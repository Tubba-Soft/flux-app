//! Per-application blocking through user-mode Windows Filtering Platform filters.
//!
//! This is the same mechanism Windows Firewall itself uses: a filter at the ALE
//! (Application Layer Enforcement) layers, matched on the executable's app ID, that makes
//! the kernel refuse the connection as it is created. No packet is captured, removed from
//! the stack, or re-injected, so a block costs nothing at runtime and cannot disturb a VPN
//! tunnel the way `WinDivertSend` can.
//!
//! The engine is opened with `FWPM_SESSION_FLAG_DYNAMIC`: every object added here is owned
//! by the session and is torn down by Windows when the process exits or crashes. A user can
//! never be left with an app silently blocked by a Flux that is no longer running.
//!
//! An ALE connect filter only stops *new* connections. Callers that need a block to bite
//! immediately must also tear down the sockets the process already has open.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use log::{info, warn};
use parking_lot::Mutex;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::*;
use windows_sys::Win32::System::Rpc::RPC_C_AUTHN_WINNT;

use crate::net::vpn_detect::is_vpn_process_path;
use crate::process::win32_shell::get_process_name_from_path;

/// Private sublayer so our filters never collide with another product's.
/// {7A3B1C40-9E2D-4F58-B6A1-0C5E8D741F23}
const FLUX_SUBLAYER: GUID = GUID {
    data1: 0x7A3B_1C40,
    data2: 0x9E2D,
    data3: 0x4F58,
    data4: [0xB6, 0xA1, 0x0C, 0x5E, 0x8D, 0x74, 0x1F, 0x23],
};

/// Mid-range weight. Within a sublayer the heaviest matching filter wins, so the permits
/// below must outrank the block that sits beside them.
const SUBLAYER_WEIGHT: u16 = 0x8000;
const BLOCK_WEIGHT: u64 = 1;
const PERMIT_WEIGHT: u64 = 10;

/// Layers a block is installed at: outbound connect and inbound accept, both families.
const BLOCK_LAYERS: &[GUID] = &[
    FWPM_LAYER_ALE_AUTH_CONNECT_V4,
    FWPM_LAYER_ALE_AUTH_CONNECT_V6,
    FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4,
    FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6,
];

/// Permits are only meaningful where the connection originates from this machine; a DNS
/// reply to an outbound UDP socket never reaches ALE_AUTH_RECV_ACCEPT.
const PERMIT_LAYERS: &[GUID] = &[
    FWPM_LAYER_ALE_AUTH_CONNECT_V4,
    FWPM_LAYER_ALE_AUTH_CONNECT_V6,
];

/// Infrastructure the machine cannot lose: DNS, DHCP, NTP, mDNS, LLMNR, NetBIOS.
/// Mirrors `CRITICAL_PORTS` in the shaper, which enforces the same carve-out on the packet
/// path when the engine happens to be intercepting.
const CRITICAL_PORTS: &[u16] = &[53, 67, 68, 123, 5353, 5355, 137, 138, 139];

/// Blocking one of these outright takes the whole machine off the network, so the critical
/// ports above are explicitly permitted back.
fn is_critical_system_process(path: &str) -> bool {
    matches!(
        get_process_name_from_path(path).to_lowercase().as_str(),
        "svchost.exe" | "lsass.exe" | "csrss.exe" | "services.exe" | "wininit.exe" | "system"
    )
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// Installs and removes per-application WFP block filters.
///
/// Construction never fails the caller: without administrator rights `FwpmEngineOpen0`
/// is refused and the blocker reports itself inactive, leaving the packet engine to fall
/// back to dropping packets as before.
pub struct WfpBlocker {
    engine: Option<HANDLE>,
    /// Lowercased executable path -> every filter id installed for it.
    filters: Mutex<HashMap<String, Vec<u64>>>,
    /// Paths that should be blocked but which WFP could not carry. Their blocks are only
    /// real if the packet engine intercepts, so the engine consults this.
    uncovered: Mutex<HashSet<String>>,
}

// The engine handle is an opaque kernel handle, safe to use from any thread; every call
// that touches `filters` goes through the mutex.
unsafe impl Send for WfpBlocker {}
unsafe impl Sync for WfpBlocker {}

impl WfpBlocker {
    pub fn new() -> Self {
        match Self::open_engine() {
            Ok(engine) => {
                info!("[WFP] Engine opened, per-app blocking runs without packet interception");
                Self {
                    engine: Some(engine),
                    filters: Mutex::new(HashMap::new()),
                    uncovered: Mutex::new(HashSet::new()),
                }
            }
            Err(e) => {
                warn!("[WFP] {} - falling back to packet-level blocking", e);
                Self {
                    engine: None,
                    filters: Mutex::new(HashMap::new()),
                    uncovered: Mutex::new(HashSet::new()),
                }
            }
        }
    }

    /// True when blocks are enforced by the kernel and the packet engine does not need to
    /// intercept in order to honour them.
    pub fn is_active(&self) -> bool {
        self.engine.is_some()
    }

    /// True when at least one application that should be blocked is not covered by a
    /// kernel filter. The packet engine must intercept to make those blocks real.
    pub fn has_uncovered_blocks(&self) -> bool {
        !self.uncovered.lock().is_empty()
    }

    fn open_engine() -> Result<HANDLE, String> {
        unsafe {
            let mut name = wide("Flux");
            let mut desc = wide("Flux dynamic filtering session");

            let mut session: FWPM_SESSION0 = std::mem::zeroed();
            session.displayData.name = name.as_mut_ptr();
            session.displayData.description = desc.as_mut_ptr();
            // Everything we add dies with this process, however it exits.
            session.flags = FWPM_SESSION_FLAG_DYNAMIC;

            let mut engine: HANDLE = ptr::null_mut();
            let rc = FwpmEngineOpen0(
                ptr::null(),
                RPC_C_AUTHN_WINNT,
                ptr::null(),
                &session,
                &mut engine,
            );
            if rc != ERROR_SUCCESS {
                return Err(format!("FwpmEngineOpen0 failed (0x{:08X})", rc));
            }

            let mut sub_name = wide("Flux Application Blocking");
            let mut sublayer: FWPM_SUBLAYER0 = std::mem::zeroed();
            sublayer.subLayerKey = FLUX_SUBLAYER;
            sublayer.displayData.name = sub_name.as_mut_ptr();
            sublayer.weight = SUBLAYER_WEIGHT;

            let rc = FwpmSubLayerAdd0(engine, &sublayer, ptr::null_mut());
            if rc != ERROR_SUCCESS {
                FwpmEngineClose0(engine);
                return Err(format!("FwpmSubLayerAdd0 failed (0x{:08X})", rc));
            }

            Ok(engine)
        }
    }

    /// Apply or lift a block for one executable. Idempotent: the existing filters for the
    /// path are always removed first, so repeated calls cannot accumulate duplicates.
    pub fn set_blocked(&self, path: &str, blocked: bool) -> Result<(), String> {
        let Some(engine) = self.engine else {
            // is_active() already tells the engine to intercept; no need to also track
            // every path as uncovered.
            return Err("WFP engine unavailable".to_string());
        };

        // Blocking the tunnel endpoint takes every other application offline with it. The
        // shaper refuses the same rule; this is the second half of that guard, because a
        // kernel filter would survive the shaper being bypassed.
        if blocked && is_vpn_process_path(path) {
            // Not "uncovered": this block must not be enforced anywhere. The shaper
            // refuses it too, so there is nothing for the packet path to fall back to.
            return Err(format!(
                "{} terminates a VPN tunnel; blocking it would disconnect the machine",
                path
            ));
        }

        let key = path.to_lowercase();
        let mut filters = self.filters.lock();

        if let Some(ids) = filters.remove(&key) {
            for id in ids {
                unsafe {
                    let rc = FwpmFilterDeleteById0(engine, id);
                    if rc != ERROR_SUCCESS {
                        warn!("[WFP] FwpmFilterDeleteById0({}) failed (0x{:08X})", id, rc);
                    }
                }
            }
        }

        if !blocked {
            self.uncovered.lock().remove(&key);
            return Ok(());
        }

        match unsafe { Self::install_block(engine, path) } {
            Ok(ids) => {
                info!("[WFP] Blocked {} with {} filters", path, ids.len());
                filters.insert(key.clone(), ids);
                self.uncovered.lock().remove(&key);
                Ok(())
            }
            Err(e) => {
                self.uncovered.lock().insert(key);
                Err(e)
            }
        }
    }

    /// Install the block for `path` and return every filter id created.
    ///
    /// On any failure the filters added so far are rolled back, so a partial block — which
    /// for a system process could mean "blocked but DNS not permitted back" — is never left
    /// in place.
    unsafe fn install_block(engine: HANDLE, path: &str) -> Result<Vec<u64>, String> {
        let file = wide(path);
        let mut app_id: *mut FWP_BYTE_BLOB = ptr::null_mut();
        let rc = FwpmGetAppIdFromFileName0(file.as_ptr(), &mut app_id);
        if rc != ERROR_SUCCESS || app_id.is_null() {
            return Err(format!(
                "FwpmGetAppIdFromFileName0 failed for {} (0x{:08X})",
                path, rc
            ));
        }

        let mut ids: Vec<u64> = Vec::new();
        let result = Self::add_all_filters(engine, path, app_id, &mut ids);
        FwpmFreeMemory0(&mut app_id as *mut _ as *mut *mut core::ffi::c_void);

        match result {
            Ok(()) => Ok(ids),
            Err(e) => {
                for id in ids {
                    FwpmFilterDeleteById0(engine, id);
                }
                Err(e)
            }
        }
    }

    unsafe fn add_all_filters(
        engine: HANDLE,
        path: &str,
        app_id: *mut FWP_BYTE_BLOB,
        ids: &mut Vec<u64>,
    ) -> Result<(), String> {
        // Permits go in first so that no window exists where the block is live but the
        // carve-out for DNS and DHCP is not.
        if is_critical_system_process(path) {
            for layer in PERMIT_LAYERS {
                for port in CRITICAL_PORTS {
                    ids.push(Self::add_filter(
                        engine,
                        *layer,
                        app_id,
                        Some(*port),
                        FWP_ACTION_PERMIT,
                        PERMIT_WEIGHT,
                        "Flux allow critical port",
                    )?);
                }
            }
        }

        for layer in BLOCK_LAYERS {
            ids.push(Self::add_filter(
                engine,
                *layer,
                app_id,
                None,
                FWP_ACTION_BLOCK,
                BLOCK_WEIGHT,
                "Flux block application",
            )?);
        }

        Ok(())
    }

    unsafe fn add_filter(
        engine: HANDLE,
        layer: GUID,
        app_id: *mut FWP_BYTE_BLOB,
        remote_port: Option<u16>,
        action: FWP_ACTION_TYPE,
        weight: u64,
        label: &str,
    ) -> Result<u64, String> {
        let mut conditions: Vec<FWPM_FILTER_CONDITION0> = Vec::with_capacity(2);

        let mut app_cond: FWPM_FILTER_CONDITION0 = std::mem::zeroed();
        app_cond.fieldKey = FWPM_CONDITION_ALE_APP_ID;
        app_cond.matchType = FWP_MATCH_EQUAL;
        app_cond.conditionValue.r#type = FWP_BYTE_BLOB_TYPE;
        app_cond.conditionValue.Anonymous.byteBlob = app_id;
        conditions.push(app_cond);

        if let Some(port) = remote_port {
            let mut port_cond: FWPM_FILTER_CONDITION0 = std::mem::zeroed();
            port_cond.fieldKey = FWPM_CONDITION_IP_REMOTE_PORT;
            port_cond.matchType = FWP_MATCH_EQUAL;
            port_cond.conditionValue.r#type = FWP_UINT16;
            port_cond.conditionValue.Anonymous.uint16 = port;
            conditions.push(port_cond);
        }

        let mut name = wide(label);
        // `weight` is held by pointer inside FWP_VALUE0, so it must outlive the add call.
        let weight_value = weight;

        let mut filter: FWPM_FILTER0 = std::mem::zeroed();
        filter.displayData.name = name.as_mut_ptr();
        filter.layerKey = layer;
        filter.subLayerKey = FLUX_SUBLAYER;
        filter.weight.r#type = FWP_UINT64;
        filter.weight.Anonymous.uint64 = &weight_value as *const u64 as *mut u64;
        filter.numFilterConditions = conditions.len() as u32;
        filter.filterCondition = conditions.as_mut_ptr();
        filter.action.r#type = action;

        let mut id: u64 = 0;
        let rc = FwpmFilterAdd0(engine, &filter, ptr::null_mut(), &mut id);
        if rc != ERROR_SUCCESS {
            return Err(format!("FwpmFilterAdd0 failed (0x{:08X})", rc));
        }
        Ok(id)
    }
}

impl Drop for WfpBlocker {
    fn drop(&mut self) {
        if let Some(engine) = self.engine.take() {
            // Closing a dynamic session removes the sublayer and every filter with it.
            unsafe { FwpmEngineClose0(engine) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_hosts_are_recognised_case_insensitively() {
        assert!(is_critical_system_process(r"C:\Windows\System32\svchost.exe"));
        assert!(is_critical_system_process(r"C:\WINDOWS\SYSTEM32\SvcHost.EXE"));
        assert!(is_critical_system_process(r"C:\Windows\System32\lsass.exe"));
    }

    #[test]
    fn ordinary_apps_are_not_system_hosts() {
        assert!(!is_critical_system_process(r"C:\Program Files\Chrome\chrome.exe"));
        assert!(!is_critical_system_process(r"D:\games\svchost_fake\game.exe"));
    }

    #[test]
    fn wide_is_null_terminated() {
        assert_eq!(wide("ab"), vec![97u16, 98, 0]);
        assert_eq!(wide(""), vec![0u16]);
    }

    #[test]
    fn a_blocked_system_process_gets_a_permit_for_every_critical_port() {
        // 2 connect layers x 9 ports permitted, plus 4 block layers.
        assert_eq!(PERMIT_LAYERS.len() * CRITICAL_PORTS.len() + BLOCK_LAYERS.len(), 22);
        assert!(PERMIT_WEIGHT > BLOCK_WEIGHT, "permits must outrank the block");
    }
}
