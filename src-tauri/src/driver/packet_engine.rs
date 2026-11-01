use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use log::{info, warn, debug};

use crate::driver::windivert_ffi::*;
use crate::driver::sni::{extract_sni_from_payload, lookup_domain, record_domain_for_ip};
use crate::net::ip_helper::IpHelperResolver;
use crate::net::vpn_detect::detect_vpn_endpoints;
use crate::net::wfp_block::WfpBlocker;
use crate::process::win32_shell::{get_process_name_from_path, get_process_path};
use crate::shaper::token_bucket::{ShaperDecision, TokenBucketShaper};
use crate::storage::db::Database;
use crate::alerts::app_detector::AppDetector;

#[inline]
fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// How the engine touches the network stack.
///
/// `Sniff` opens WinDivert with SNIFF|RECV_ONLY: the driver hands us a *copy* of each packet
/// and leaves the original in the stack. Nothing is removed, re-routed or re-injected, so a
/// VPN tunnel cannot be disturbed. `Intercept` is the classic divert mode and is only needed
/// once the user has an actual block or rate limit in place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EngineMode {
    Sniff,
    Intercept,
}

impl EngineMode {
    pub fn label(self) -> &'static str {
        match self {
            EngineMode::Sniff => "kernel_monitor",
            EngineMode::Intercept => "kernel_shaper",
        }
    }
}

/// Build the WinDivert filter, excluding every VPN server endpoint.
///
/// Two constraints shape this, both verified against WinDivert's own filter compiler
/// rather than assumed:
///
/// * `not (...)` over a parenthesised expression does not parse. WinDivert only negates a
///   single test, as in `!loopback`, so the exclusion has to be a chain of `!=`.
/// * A test on a field the packet does not carry evaluates to *false*, so a bare
///   `ip.SrcAddr != X` would quietly exclude every IPv6 packet from the filter.
///
/// Hence the address family is selected first and the exclusions applied within it: each
/// family keeps its own packets and only drops its own listed endpoints.
fn build_filter(vpn_endpoints: &[IpAddr]) -> String {
    let mut filter = String::from("!loopback and (tcp or udp)");
    if vpn_endpoints.is_empty() {
        return filter;
    }

    fn clause(proto: &str, addrs: impl Iterator<Item = IpAddr>) -> String {
        let terms: Vec<String> = addrs
            .flat_map(|ip| {
                [
                    format!("{}.SrcAddr != {}", proto, ip),
                    format!("{}.DstAddr != {}", proto, ip),
                ]
            })
            .collect();
        if terms.is_empty() {
            // No endpoint of this family to exclude: admit all of its packets.
            proto.to_string()
        } else {
            format!("({} and {})", proto, terms.join(" and "))
        }
    }

    let v4 = clause("ip", vpn_endpoints.iter().copied().filter(IpAddr::is_ipv4));
    let v6 = clause("ipv6", vpn_endpoints.iter().copied().filter(IpAddr::is_ipv6));

    filter.push_str(&format!(" and ({} or {})", v4, v6));
    filter
}

/// Upper bound on ports expressible in a WinDivert filter before falling back to
/// diverting everything. Each port costs two terms and the compiler caps filter size.
const MAX_SHAPED_PORTS: usize = 40;

/// The two filters the engine should currently be running.
///
/// `sniff` watches traffic without touching it; `intercept` is the only traffic that is
/// pulled out of the stack and re-injected. Keeping them disjoint means a rule on one
/// application cannot put anyone else's packets on the interception path - which is what
/// made a single forgotten rate limit break the machine's networking.
#[derive(Clone, PartialEq, Eq)]
struct EnginePlan {
    sniff: Option<String>,
    intercept: Option<String>,
}

impl EnginePlan {
    fn describe(&self) -> String {
        match (&self.sniff, &self.intercept) {
            (Some(s), None) => format!("sniff only: {}", s),
            (None, Some(i)) => format!("intercept all: {}", i),
            (Some(s), Some(i)) => format!("sniff: {} | intercept: {}", s, i),
            (None, None) => "idle".to_string(),
        }
    }
}

/// Local port carried by a stream id of the form `PROTO-local_ip:port-remote_ip:port`.
fn local_port_of_stream(stream_id: &str) -> Option<u16> {
    // Addresses never contain '-', so the three fields split cleanly even for IPv6.
    let local = stream_id.split('-').nth(1)?;
    local.rsplit(':').next()?.parse().ok()
}

struct DelayedPacket {
    send_at: Instant,
    data: Vec<u8>,
    addr: WinDivertAddress,
}

pub struct StreamStats {
    pub stream_id: String,
    pub protocol: String,
    pub local_ip: IpAddr,
    pub local_port: u16,
    pub remote_ip: IpAddr,
    pub remote_port: u16,
    pub remote_domain: Option<String>,
    pub pid: u32,
    pub process_path: Option<String>,
    pub down_bytes_last_window: AtomicU64,
    pub up_bytes_last_window: AtomicU64,
    pub total_down_bytes: AtomicU64,
    pub total_up_bytes: AtomicU64,
    pub down_speed_bps: AtomicU64,
    pub up_speed_bps: AtomicU64,
    pub last_seen_ms: AtomicU64,
}

pub struct ProcessStats {
    pub pid: u32,
    pub path: String,
    pub name: String,
    pub down_bytes_last_window: AtomicU64,
    pub up_bytes_last_window: AtomicU64,
    pub total_down_bytes: AtomicU64,
    pub total_up_bytes: AtomicU64,
    pub down_speed_bps: AtomicU64,
    pub up_speed_bps: AtomicU64,
    pub active_stream_ids: DashMap<String, ()>,
}

pub struct PacketEngine {
    divert_lib: Option<WinDivertLib>,
    /// Watches everything, re-injects nothing.
    sniff_handle: Arc<RwLock<Option<SafeWinDivertHandle>>>,
    /// Carries only the ports currently under a rule.
    intercept_handle: Arc<RwLock<Option<SafeWinDivertHandle>>>,
    /// Bumped on every (re)start so worker threads from a previous generation exit instead
    /// of racing the new ones for the same handle.
    generation: Arc<AtomicU64>,
    active_mode: Arc<RwLock<Option<EngineMode>>>,
    active_plan: Arc<RwLock<Option<EnginePlan>>>,
    /// Serialises stop+start so a rule change and the periodic VPN re-check cannot both
    /// open a handle and leak one of them.
    reconcile_lock: Arc<Mutex<()>>,
    pub is_running: Arc<AtomicBool>,
    pub packet_counter: Arc<AtomicU64>,
    pub byte_counter: Arc<AtomicU64>,
    pub global_down_bytes_window: Arc<AtomicU64>,
    pub global_up_bytes_window: Arc<AtomicU64>,
    pub global_total_down_bytes: Arc<AtomicU64>,
    pub global_total_up_bytes: Arc<AtomicU64>,
    pub global_down_speed_bps: Arc<AtomicU64>,
    pub global_up_speed_bps: Arc<AtomicU64>,
    pub streams: Arc<DashMap<String, Arc<StreamStats>>>,
    pub processes: Arc<DashMap<u32, Arc<ProcessStats>>>,
    ip_resolver: Arc<IpHelperResolver>,
    shaper: Arc<TokenBucketShaper>,
    wfp: Arc<WfpBlocker>,
    db: Arc<Database>,
    detector: Arc<AppDetector>,
}

impl PacketEngine {
    pub fn new(
        ip_resolver: Arc<IpHelperResolver>,
        shaper: Arc<TokenBucketShaper>,
        wfp: Arc<WfpBlocker>,
        db: Arc<Database>,
        detector: Arc<AppDetector>,
    ) -> Self {
        let divert_lib = match WinDivertLib::load() {
            Ok(lib) => Some(lib),
            Err(e) => {
                warn!("WinDivertLib dynamic load note: {}", e);
                None
            }
        };

        Self {
            divert_lib,
            sniff_handle: Arc::new(RwLock::new(None)),
            intercept_handle: Arc::new(RwLock::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            active_mode: Arc::new(RwLock::new(None)),
            active_plan: Arc::new(RwLock::new(None)),
            reconcile_lock: Arc::new(Mutex::new(())),
            is_running: Arc::new(AtomicBool::new(false)),
            packet_counter: Arc::new(AtomicU64::new(0)),
            byte_counter: Arc::new(AtomicU64::new(0)),
            global_down_bytes_window: Arc::new(AtomicU64::new(0)),
            global_up_bytes_window: Arc::new(AtomicU64::new(0)),
            global_total_down_bytes: Arc::new(AtomicU64::new(0)),
            global_total_up_bytes: Arc::new(AtomicU64::new(0)),
            global_down_speed_bps: Arc::new(AtomicU64::new(0)),
            global_up_speed_bps: Arc::new(AtomicU64::new(0)),
            streams: Arc::new(DashMap::new()),
            processes: Arc::new(DashMap::new()),
            ip_resolver,
            shaper,
            wfp,
            db,
            detector,
        }
    }

    /// Open the engine for the current rule set.
    pub fn start(&self) -> Result<(), String> {
        let _guard = self.reconcile_lock.lock();
        let plan = self.build_plan();
        self.apply_plan(plan)
    }

    pub fn active_mode(&self) -> Option<EngineMode> {
        *self.active_mode.read()
    }

    pub fn active_filter(&self) -> String {
        self.active_plan
            .read()
            .as_ref()
            .map(|p| p.describe())
            .unwrap_or_default()
    }

    /// Local ports belonging to traffic that is actually under a rule right now.
    ///
    /// A rule names an executable, but WinDivert's network layer sees only addresses and
    /// ports, so the rule has to be translated into the ports its process currently holds.
    /// A limited application that is not running contributes nothing, which is the whole
    /// point: its rule must not pull anyone else's packets onto the interception path.
    fn shaped_ports(&self) -> Vec<(bool, u16)> {
        let wfp_carries_blocks = self.wfp.is_active() && !self.wfp.has_uncovered_blocks();

        let mut paths: Vec<String> = self.shaper.rate_limited_paths();
        if !wfp_carries_blocks {
            paths.extend(self.shaper.blocked_path_list());
        }

        let mut ports: Vec<(bool, u16)> = Vec::new();

        if !paths.is_empty() {
            let wanted: Vec<String> = paths.iter().map(|p| p.to_lowercase()).collect();
            for socket in self.ip_resolver.get_active_sockets() {
                if socket.pid <= 4 || socket.local_port == 0 {
                    continue;
                }
                let Some(owner) = get_process_path(socket.pid) else {
                    continue;
                };
                if wanted.contains(&owner.to_lowercase()) {
                    ports.push((socket.protocol == "TCP", socket.local_port));
                }
            }
        }

        // A rule on one connection is already expressed as a 5-tuple; its local port is
        // enough to put that connection, and only it, on the interception path.
        for id in self.shaper.ruled_stream_ids() {
            if let Some(port) = local_port_of_stream(&id) {
                ports.push((id.starts_with("TCP"), port));
            }
        }

        ports.sort();
        ports.dedup();
        ports
    }

    fn build_plan(&self) -> EnginePlan {
        let base = build_filter(&detect_vpn_endpoints(&self.ip_resolver));
        let ports = self.shaped_ports();

        if ports.is_empty() {
            // Nothing to shape: watch everything, touch nothing.
            return EnginePlan { sniff: Some(base), intercept: None };
        }

        if ports.len() > MAX_SHAPED_PORTS {
            // Too many ports to express as a filter. Diverting everything is the only way
            // left to honour the rules, so do that rather than enforce them partially.
            warn!(
                "{} shaped ports exceeds the filter budget; diverting all traffic",
                ports.len()
            );
            return EnginePlan { sniff: None, intercept: Some(base) };
        }

        let terms: Vec<String> = ports
            .iter()
            .flat_map(|(is_tcp, port)| {
                let proto = if *is_tcp { "tcp" } else { "udp" };
                [
                    format!("{}.SrcPort == {}", proto, port),
                    format!("{}.DstPort == {}", proto, port),
                ]
            })
            .collect();
        let predicate = terms.join(" or ");

        EnginePlan {
            sniff: Some(format!("{} and not ({})", base, predicate)),
            intercept: Some(format!("{} and ({})", base, predicate)),
        }
    }

    /// Re-open the engine when the plan has changed.
    ///
    /// Called after every rule change and on a tick, so that a VPN connecting, or a limited
    /// application opening a new connection, is picked up. A no-op when nothing moved.
    pub fn reconcile(&self) -> Result<(), String> {
        let _guard = self.reconcile_lock.lock();
        if self.divert_lib.is_none() {
            return Ok(());
        }

        let plan = self.build_plan();
        let running = self.sniff_handle.read().is_some() || self.intercept_handle.read().is_some();
        if running && self.active_plan.read().as_ref() == Some(&plan) {
            return Ok(());
        }

        info!("Reconciling packet engine -> {}", plan.describe());
        self.stop();
        self.apply_plan(plan)
    }

    fn apply_plan(&self, plan: EnginePlan) -> Result<(), String> {
        let lib = match &self.divert_lib {
            Some(l) => l.clone(),
            None => return Err("WinDivert library not loaded".to_string()),
        };

        self.is_running.store(true, Ordering::SeqCst);
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

        // Telemetry. SNIFF|RECV_ONLY hands over copies and leaves the originals in the
        // stack: no re-injection, no fresh route lookup, nothing a VPN tunnel can trip on.
        if let Some(filter) = &plan.sniff {
            let handle = self.open_handle(
                &lib,
                filter,
                WINDIVERT_FLAG_SNIFF | WINDIVERT_FLAG_RECV_ONLY,
                &plan,
            )?;
            *self.sniff_handle.write() = Some(handle);
            self.spawn_packet_loop(lib.clone(), self.sniff_handle.clone(), false, generation);
        }

        // Shaping. Only the ports named in the plan reach this handle.
        if let Some(filter) = &plan.intercept {
            let handle = self.open_handle(&lib, filter, 0, &plan)?;
            *self.intercept_handle.write() = Some(handle);
            self.spawn_packet_loop(lib.clone(), self.intercept_handle.clone(), true, generation);
        }

        *self.active_mode.write() = Some(if plan.intercept.is_some() {
            EngineMode::Intercept
        } else {
            EngineMode::Sniff
        });
        info!("Packet engine running -> {}", plan.describe());
        *self.active_plan.write() = Some(plan);
        Ok(())
    }

    fn open_handle(
        &self,
        lib: &WinDivertLib,
        filter: &str,
        flags: u64,
        plan: &EnginePlan,
    ) -> Result<SafeWinDivertHandle, String> {
        // Losing all monitoring because one endpoint produced a filter WinDivert dislikes
        // would be worse than losing the VPN exclusion, so retry without it.
        let handle = match lib.open(filter, WINDIVERT_LAYER_NETWORK, 0, flags) {
            Ok(h) => h,
            Err(e) => {
                let base = build_filter(&[]);
                if filter == base || plan.intercept.is_some() {
                    return Err(e);
                }
                warn!("WinDivert rejected '{}' ({}), falling back to '{}'", filter, e, base);
                lib.open(&base, WINDIVERT_LAYER_NETWORK, 0, flags)?
            }
        };

        // Queue length and time prevent packet drops under heavy 100+ Mbps traffic
        lib.set_param(handle, WINDIVERT_PARAM_QUEUE_LENGTH, 16384);
        lib.set_param(handle, WINDIVERT_PARAM_QUEUE_TIME, 2000); // 2000 ms queue buffer
        lib.set_param(handle, WINDIVERT_PARAM_QUEUE_SIZE, 32 * 1024 * 1024); // 32 MB buffer
        Ok(handle)
    }

    fn spawn_packet_loop(
        &self,
        lib: WinDivertLib,
        handle_arc: Arc<RwLock<Option<SafeWinDivertHandle>>>,
        intercepting: bool,
        generation: u64,
    ) {
        let is_running = self.is_running.clone();
        let gen_counter = self.generation.clone();
        let packet_counter = self.packet_counter.clone();
        let byte_counter = self.byte_counter.clone();
        let global_down_bytes_window = self.global_down_bytes_window.clone();
        let global_up_bytes_window = self.global_up_bytes_window.clone();
        let global_total_down_bytes = self.global_total_down_bytes.clone();
        let global_total_up_bytes = self.global_total_up_bytes.clone();
        let streams = self.streams.clone();
        let processes = self.processes.clone();
        let ip_resolver = self.ip_resolver.clone();
        let shaper = self.shaper.clone();
        let detector = self.detector.clone();

        // The delayed-packet worker only exists where we actually re-inject.
        let delay_tx = if intercepting {
            let (delay_tx, delay_rx) = std::sync::mpsc::sync_channel::<DelayedPacket>(16384);
            let delay_lib = lib.clone();
            let is_running_delay = self.is_running.clone();
            let delay_gen = self.generation.clone();
            let delay_handle_arc = handle_arc.clone();

            std::thread::spawn(move || {
                while is_running_delay.load(Ordering::Relaxed)
                    && delay_gen.load(Ordering::Relaxed) == generation
                {
                    match delay_rx.recv_timeout(Duration::from_millis(10)) {
                        Ok(pkt) => {
                            // Read handle from Arc so we see stop() invalidation immediately
                            let h = match *delay_handle_arc.read() {
                                Some(h) => h,
                                None => continue, // Handle closed by stop(), skip this packet
                            };
                            let now = Instant::now();
                            if pkt.send_at > now {
                                std::thread::sleep(pkt.send_at - now);
                            }
                            let _ = delay_lib.send(h, &pkt.data, &pkt.addr);
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
            Some(delay_tx)
        } else {
            None
        };

        std::thread::spawn(move || {
            let role = if intercepting { "intercept" } else { "sniff" };
            info!("WinDivert {} loop started (generation {})", role, generation);

            let mut packet_buf = vec![0u8; 65535];
            let mut addr = WinDivertAddress::new();

            while is_running.load(Ordering::Relaxed)
                && gen_counter.load(Ordering::Relaxed) == generation
            {
                // Read handle from Arc so we see stop() invalidation immediately
                let current_handle = match *handle_arc.read() {
                    Some(h) => h,
                    None => break, // Handle closed by stop(), exit loop cleanly
                };
                match lib.recv(current_handle, &mut packet_buf, &mut addr) {
                    Ok(packet_len) => {
                        packet_counter.fetch_add(1, Ordering::Relaxed);
                        byte_counter.fetch_add(packet_len as u64, Ordering::Relaxed);

                        let packet_slice = &mut packet_buf[..packet_len];
                        let is_outbound = addr.is_outbound();

                        if is_outbound {
                            global_up_bytes_window.fetch_add(packet_len as u64, Ordering::Relaxed);
                            global_total_up_bytes.fetch_add(packet_len as u64, Ordering::Relaxed);
                        } else {
                            global_down_bytes_window.fetch_add(packet_len as u64, Ordering::Relaxed);
                            global_total_down_bytes.fetch_add(packet_len as u64, Ordering::Relaxed);
                        }

                        // Parse IP and 5-tuple
                        if let Some((proto_name, local_ip, local_port, remote_ip, remote_port, payload_offset)) =
                            parse_packet_5tuple(packet_slice, is_outbound)
                        {
                            let stream_id = format!(
                                "{}-{}:{}-{}:{}",
                                proto_name, local_ip, local_port, remote_ip, remote_port
                            );

                            // Extract SNI if TLS on port 443 / TCP
                            let mut sni_domain = None;
                            if proto_name == "TCP" && payload_offset < packet_len {
                                if let Some(sni) = extract_sni_from_payload(&packet_slice[payload_offset..]) {
                                    record_domain_for_ip(remote_ip, sni.clone());
                                    sni_domain = Some(sni);
                                }
                            }
                            if sni_domain.is_none() {
                                sni_domain = lookup_domain(&remote_ip);
                            }

                            // Resolve PID using concurrent IP Helper resolver with UDP TTL cache
                            let is_tcp = proto_name == "TCP";
                            let pid = ip_resolver.resolve_pid(is_tcp, local_port, remote_port).unwrap_or(0);
                            let exe_path = get_process_path(pid);

                            // Alert on new network-active process
                            if let Some(ref path) = exe_path {
                                if pid > 4 {
                                    let name = get_process_name_from_path(path);
                                    detector.check_new_app(pid, &name, path, &remote_ip.to_string(), remote_port, proto_name);
                                }
                            }

                            // Record telemetry
                            record_packet_metrics(
                                &streams,
                                &processes,
                                &stream_id,
                                proto_name,
                                local_ip,
                                local_port,
                                remote_ip,
                                remote_port,
                                sni_domain,
                                pid,
                                exe_path.as_deref(),
                                is_outbound,
                                packet_len as u64,
                            );

                            // Sniffed packets are copies; the originals never left the stack.
                            let delay_tx = match delay_tx.as_ref() {
                                Some(tx) => tx,
                                None => continue,
                            };

                            // Token Bucket evaluation (with port-aware critical system protection)
                            let decision = shaper.evaluate_packet_with_ports(
                                exe_path.as_deref(),
                                &stream_id,
                                is_outbound,
                                packet_len,
                                local_port,
                                remote_port,
                            );

                            match decision {
                                ShaperDecision::Drop => {
                                    // Blocked/Dropped: Do not reinject
                                    continue;
                                }
                                ShaperDecision::Delay(wait_time) => {
                                    // Queue packet for asynchronous delayed pacing without blocking the main loop
                                    let delayed = DelayedPacket {
                                        send_at: Instant::now() + wait_time,
                                        data: packet_slice.to_vec(),
                                        addr,
                                    };
                                    if let Err(_) = delay_tx.try_send(delayed) {
                                        // SAFETY: Send packet immediately instead of dropping!
                                        // Dropping packets causes phantom internet outages.
                                        warn!("Delay queue full, sending packet immediately to prevent internet outage");
                                        let _ = lib.send(current_handle, packet_slice, &addr);
                                    }
                                }
                                ShaperDecision::Pass => {
                                    if let Err(e) = lib.send(current_handle, packet_slice, &addr) {
                                        warn!("WinDivert send error: {}", e);
                                    }
                                }
                            }
                        } else if intercepting {
                            if let Err(e) = lib.send(current_handle, packet_slice, &addr) {
                                warn!("WinDivert send (passthrough) error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        if is_running.load(Ordering::Relaxed)
                            && gen_counter.load(Ordering::Relaxed) == generation
                        {
                            warn!("WinDivert {} recv error: {}", role, e);
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                }
            }

            // Handles are closed by stop() only - do NOT close here to prevent double-close!
            info!("WinDivert {} loop stopped cleanly", role);
        });
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.active_mode.write() = None;
        *self.active_plan.write() = None;

        let lib = self.divert_lib.clone();
        for slot in [&self.sniff_handle, &self.intercept_handle] {
            if let Some(h) = slot.write().take() {
                if let Some(lib) = &lib {
                    lib.close(h);
                }
            }
        }
    }

    pub fn compute_speed_window(&self, elapsed_secs: f64) {
        let multiplier = if elapsed_secs > 0.0 { 1.0 / elapsed_secs } else { 1.0 };

        // 1. Calculate and smooth true global download & upload speeds
        let g_down_recent = self.global_down_bytes_window.swap(0, Ordering::Relaxed);
        let g_up_recent = self.global_up_bytes_window.swap(0, Ordering::Relaxed);
        let g_inst_down = (g_down_recent as f64 * multiplier) as u64;
        let g_inst_up = (g_up_recent as f64 * multiplier) as u64;

        let g_prev_down = self.global_down_speed_bps.load(Ordering::Relaxed);
        let g_prev_up = self.global_up_speed_bps.load(Ordering::Relaxed);

        let g_smoothed_down = if g_inst_down > 0 {
            (g_prev_down as f64 * 0.25 + g_inst_down as f64 * 0.75) as u64
        } else if g_prev_down > 100 {
            (g_prev_down as f64 * 0.55) as u64
        } else {
            0
        };

        let g_smoothed_up = if g_inst_up > 0 {
            (g_prev_up as f64 * 0.25 + g_inst_up as f64 * 0.75) as u64
        } else if g_prev_up > 100 {
            (g_prev_up as f64 * 0.55) as u64
        } else {
            0
        };

        self.global_down_speed_bps.store(g_smoothed_down, Ordering::Relaxed);
        self.global_up_speed_bps.store(g_smoothed_up, Ordering::Relaxed);

        for entry in self.streams.iter() {
            let stats = entry.value();
            let down_recent = stats.down_bytes_last_window.swap(0, Ordering::Relaxed);
            let up_recent = stats.up_bytes_last_window.swap(0, Ordering::Relaxed);

            let inst_down = (down_recent as f64 * multiplier) as u64;
            let inst_up = (up_recent as f64 * multiplier) as u64;

            let prev_down = stats.down_speed_bps.load(Ordering::Relaxed);
            let prev_up = stats.up_speed_bps.load(Ordering::Relaxed);

            // Responsive EMA: rapid response on active bursts, smooth decay when silent
            let smoothed_down = if inst_down > 0 {
                (prev_down as f64 * 0.25 + inst_down as f64 * 0.75) as u64
            } else if prev_down > 100 {
                (prev_down as f64 * 0.55) as u64
            } else {
                0
            };

            let smoothed_up = if inst_up > 0 {
                (prev_up as f64 * 0.25 + inst_up as f64 * 0.75) as u64
            } else if prev_up > 100 {
                (prev_up as f64 * 0.55) as u64
            } else {
                0
            };

            stats.down_speed_bps.store(smoothed_down, Ordering::Relaxed);
            stats.up_speed_bps.store(smoothed_up, Ordering::Relaxed);
        }

        for entry in self.processes.iter() {
            let stats = entry.value();
            let down_recent = stats.down_bytes_last_window.swap(0, Ordering::Relaxed);
            let up_recent = stats.up_bytes_last_window.swap(0, Ordering::Relaxed);

            let inst_down = (down_recent as f64 * multiplier) as u64;
            let inst_up = (up_recent as f64 * multiplier) as u64;

            let prev_down = stats.down_speed_bps.load(Ordering::Relaxed);
            let prev_up = stats.up_speed_bps.load(Ordering::Relaxed);

            let smoothed_down = if inst_down > 0 {
                (prev_down as f64 * 0.25 + inst_down as f64 * 0.75) as u64
            } else if prev_down > 100 {
                (prev_down as f64 * 0.55) as u64
            } else {
                0
            };

            let smoothed_up = if inst_up > 0 {
                (prev_up as f64 * 0.25 + inst_up as f64 * 0.75) as u64
            } else if prev_up > 100 {
                (prev_up as f64 * 0.55) as u64
            } else {
                0
            };

            stats.down_speed_bps.store(smoothed_down, Ordering::Relaxed);
            stats.up_speed_bps.store(smoothed_up, Ordering::Relaxed);
        }

        let now_ms = current_epoch_ms();
        self.streams.retain(|_, s| {
            now_ms.saturating_sub(s.last_seen_ms.load(Ordering::Relaxed)) < 30_000
        });
    }

    pub fn get_global_speeds(&self) -> (u64, u64, u64, u64) {
        (
            self.global_down_speed_bps.load(Ordering::Relaxed),
            self.global_up_speed_bps.load(Ordering::Relaxed),
            self.global_total_down_bytes.load(Ordering::Relaxed),
            self.global_total_up_bytes.load(Ordering::Relaxed),
        )
    }
}

fn parse_packet_5tuple(
    packet: &[u8],
    is_outbound: bool,
) -> Option<(&'static str, IpAddr, u16, IpAddr, u16, usize)> {
    if packet.is_empty() {
        return None;
    }

    let version = packet[0] >> 4;
    if version == 4 {
        // IPv4 Header parsing
        if packet.len() < 20 {
            return None;
        }
        let ihl = ((packet[0] & 0x0F) * 4) as usize;
        if packet.len() < ihl {
            return None;
        }

        let protocol = packet[9];
        let src_ip = IpAddr::V4(Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]));
        let dst_ip = IpAddr::V4(Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]));

        let transport_slice = &packet[ihl..];

        let (proto_str, src_port, dst_port, payload_offset) = match protocol {
            6 => {
                if transport_slice.len() < 20 {
                    return None;
                }
                let src_p = u16::from_be_bytes([transport_slice[0], transport_slice[1]]);
                let dst_p = u16::from_be_bytes([transport_slice[2], transport_slice[3]]);
                let data_offset = ((transport_slice[12] >> 4) * 4) as usize;
                ("TCP", src_p, dst_p, ihl + data_offset)
            }
            17 => {
                if transport_slice.len() < 8 {
                    return None;
                }
                let src_p = u16::from_be_bytes([transport_slice[0], transport_slice[1]]);
                let dst_p = u16::from_be_bytes([transport_slice[2], transport_slice[3]]);
                ("UDP", src_p, dst_p, ihl + 8)
            }
            _ => return None,
        };

        let (local_ip, local_port, remote_ip, remote_port) = if is_outbound {
            (src_ip, src_port, dst_ip, dst_port)
        } else {
            (dst_ip, dst_port, src_ip, src_port)
        };

        Some((proto_str, local_ip, local_port, remote_ip, remote_port, payload_offset))
    } else if version == 6 {
        // IPv6 Header parsing (Fixed 40-byte base header)
        if packet.len() < 40 {
            return None;
        }

        let next_header = packet[6];
        let mut src_bytes = [0u8; 16];
        let mut dst_bytes = [0u8; 16];
        src_bytes.copy_from_slice(&packet[8..24]);
        dst_bytes.copy_from_slice(&packet[24..40]);

        let src_ip = IpAddr::V6(Ipv6Addr::from(src_bytes));
        let dst_ip = IpAddr::V6(Ipv6Addr::from(dst_bytes));

        let mut next_header = packet[6];
        let mut offset = 40;

        while offset < packet.len() {
            match next_header {
                6 => {
                    let transport_slice = &packet[offset..];
                    if transport_slice.len() < 20 {
                        return None;
                    }
                    let src_p = u16::from_be_bytes([transport_slice[0], transport_slice[1]]);
                    let dst_p = u16::from_be_bytes([transport_slice[2], transport_slice[3]]);
                    let data_offset = ((transport_slice[12] >> 4) * 4) as usize;
                    let (local_ip, local_port, remote_ip, remote_port) = if is_outbound {
                        (src_ip, src_p, dst_ip, dst_p)
                    } else {
                        (dst_ip, dst_p, src_ip, src_p)
                    };
                    return Some(("TCP", local_ip, local_port, remote_ip, remote_port, offset + data_offset));
                }
                17 => {
                    let transport_slice = &packet[offset..];
                    if transport_slice.len() < 8 {
                        return None;
                    }
                    let src_p = u16::from_be_bytes([transport_slice[0], transport_slice[1]]);
                    let dst_p = u16::from_be_bytes([transport_slice[2], transport_slice[3]]);
                    let (local_ip, local_port, remote_ip, remote_port) = if is_outbound {
                        (src_ip, src_p, dst_ip, dst_p)
                    } else {
                        (dst_ip, dst_p, src_ip, src_p)
                    };
                    return Some(("UDP", local_ip, local_port, remote_ip, remote_port, offset + 8));
                }
                0 | 43 | 60 => {
                    if offset + 2 > packet.len() {
                        return None;
                    }
                    let ext_next = packet[offset];
                    let ext_len = (packet[offset + 1] as usize + 1) * 8;
                    next_header = ext_next;
                    offset += ext_len;
                }
                44 => {
                    if offset + 8 > packet.len() {
                        return None;
                    }
                    next_header = packet[offset];
                    offset += 8;
                }
                51 => {
                    if offset + 2 > packet.len() {
                        return None;
                    }
                    let ah_next = packet[offset];
                    let ah_len = (packet[offset + 1] as usize + 2) * 4;
                    next_header = ah_next;
                    offset += ah_len;
                }
                _ => return None,
            }
        }
        None
    } else {
        None
    }
}

fn record_packet_metrics(
    streams: &DashMap<String, Arc<StreamStats>>,
    processes: &DashMap<u32, Arc<ProcessStats>>,
    stream_id: &str,
    protocol: &'static str,
    local_ip: IpAddr,
    local_port: u16,
    remote_ip: IpAddr,
    remote_port: u16,
    domain: Option<String>,
    pid: u32,
    exe_path: Option<&str>,
    is_outbound: bool,
    bytes: u64,
) {
    let stream_stats = streams.entry(stream_id.to_string()).or_insert_with(|| {
        Arc::new(StreamStats {
            stream_id: stream_id.to_string(),
            protocol: protocol.to_string(),
            local_ip,
            local_port,
            remote_ip,
            remote_port,
            remote_domain: domain.clone(),
            pid,
            process_path: exe_path.map(|s| s.to_string()),
            down_bytes_last_window: AtomicU64::new(0),
            up_bytes_last_window: AtomicU64::new(0),
            total_down_bytes: AtomicU64::new(0),
            total_up_bytes: AtomicU64::new(0),
            down_speed_bps: AtomicU64::new(0),
            up_speed_bps: AtomicU64::new(0),
            last_seen_ms: AtomicU64::new(current_epoch_ms()),
        })
    });

    stream_stats.last_seen_ms.store(current_epoch_ms(), Ordering::Relaxed);
    if is_outbound {
        stream_stats.up_bytes_last_window.fetch_add(bytes, Ordering::Relaxed);
        stream_stats.total_up_bytes.fetch_add(bytes, Ordering::Relaxed);
    } else {
        stream_stats.down_bytes_last_window.fetch_add(bytes, Ordering::Relaxed);
        stream_stats.total_down_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    let effective_pid = pid;
    let path_str = if effective_pid == 0 {
        "System".to_string()
    } else if effective_pid == 4 {
        "C:\\Windows\\System32\\ntoskrnl.exe".to_string()
    } else {
        exe_path.unwrap_or("Unknown").to_string()
    };
    let name_str = if effective_pid == 0 {
        "System Idle / Sockets".to_string()
    } else if effective_pid == 4 {
        "System Kernel".to_string()
    } else {
        get_process_name_from_path(&path_str)
    };

    let proc_stats = processes.entry(effective_pid).or_insert_with(|| {
        Arc::new(ProcessStats {
            pid: effective_pid,
            path: path_str,
            name: name_str,
            down_bytes_last_window: AtomicU64::new(0),
            up_bytes_last_window: AtomicU64::new(0),
            total_down_bytes: AtomicU64::new(0),
            total_up_bytes: AtomicU64::new(0),
            down_speed_bps: AtomicU64::new(0),
            up_speed_bps: AtomicU64::new(0),
            active_stream_ids: DashMap::new(),
        })
    });

    proc_stats.active_stream_ids.insert(stream_id.to_string(), ());

    if is_outbound {
        proc_stats.up_bytes_last_window.fetch_add(bytes, Ordering::Relaxed);
        proc_stats.total_up_bytes.fetch_add(bytes, Ordering::Relaxed);
    } else {
        proc_stats.down_bytes_last_window.fetch_add(bytes, Ordering::Relaxed);
        proc_stats.total_down_bytes.fetch_add(bytes, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn stream_local_port_is_parsed() {
        assert_eq!(local_port_of_stream("TCP-192.168.1.5:54321-1.1.1.1:443"), Some(54321));
        assert_eq!(local_port_of_stream("UDP-10.0.0.2:53-8.8.8.8:53"), Some(53));
    }

    #[test]
    fn stream_local_port_handles_ipv6() {
        assert_eq!(local_port_of_stream("TCP-[2a07:b944::1]:8080-[::1]:443"), Some(8080));
    }

    #[test]
    fn stream_local_port_rejects_garbage() {
        assert_eq!(local_port_of_stream("nonsense"), None);
        assert_eq!(local_port_of_stream("TCP-noport-1.1.1.1:443"), None);
    }

    #[test]
    fn base_filter_when_no_vpn() {
        assert_eq!(build_filter(&[]), "!loopback and (tcp or udp)");
    }

    #[test]
    fn excludes_ipv4_endpoint_in_both_directions() {
        let ip: IpAddr = "185.159.157.1".parse().unwrap();
        assert_eq!(
            build_filter(&[ip]),
            "!loopback and (tcp or udp) and ((ip and \
ip.SrcAddr != 185.159.157.1 and ip.DstAddr != 185.159.157.1) or ipv6)"
        );
    }

    #[test]
    fn an_ipv4_exclusion_still_admits_ipv6() {
        // The bug this guards: a bare `ip.SrcAddr != X` is false for an IPv6 packet, so
        // without the family split every IPv6 packet would vanish from the filter.
        let ip: IpAddr = "10.2.0.1".parse().unwrap();
        let f = build_filter(&[ip]);
        assert!(f.ends_with("or ipv6)"), "{}", f);
    }

    #[test]
    fn uses_ipv6_field_for_ipv6_endpoint() {
        let ip: IpAddr = "2a07:b944::1".parse().unwrap();
        let f = build_filter(&[ip]);
        assert!(f.contains("ipv6.SrcAddr != 2a07:b944::1"), "{}", f);
        assert!(f.contains("ipv6.DstAddr != 2a07:b944::1"), "{}", f);
        // IPv4 carries no exclusion here, so it must be admitted wholesale.
        assert!(f.contains("(ip or "), "{}", f);
    }

    #[test]
    fn negation_of_a_group_is_never_emitted() {
        // WinDivert cannot parse `not (...)`; emitting one makes WinDivertOpen fail with
        // ERROR_INVALID_PARAMETER and the engine silently loses the exclusion.
        let a: IpAddr = "10.0.0.1".parse().unwrap();
        let b: IpAddr = "2a07:b944::1".parse().unwrap();
        for f in [build_filter(&[a]), build_filter(&[b]), build_filter(&[a, b])] {
            assert!(!f.contains("not ("), "{}", f);
            assert!(!f.contains("!("), "{}", f);
        }
    }

    #[test]
    fn multiple_endpoints_are_all_excluded() {
        let a: IpAddr = "10.0.0.1".parse().unwrap();
        let b: IpAddr = "10.0.0.2".parse().unwrap();
        assert_eq!(
            build_filter(&[a, b]),
            "!loopback and (tcp or udp) and ((ip and \
ip.SrcAddr != 10.0.0.1 and ip.DstAddr != 10.0.0.1 and \
ip.SrcAddr != 10.0.0.2 and ip.DstAddr != 10.0.0.2) or ipv6)"
        );
    }

    #[test]
    fn both_families_are_excluded_independently() {
        let a: IpAddr = "10.0.0.1".parse().unwrap();
        let b: IpAddr = "2a07:b944::1".parse().unwrap();
        assert_eq!(
            build_filter(&[a, b]),
            "!loopback and (tcp or udp) and ((ip and \
ip.SrcAddr != 10.0.0.1 and ip.DstAddr != 10.0.0.1) or (ipv6 and \
ipv6.SrcAddr != 2a07:b944::1 and ipv6.DstAddr != 2a07:b944::1))"
        );
    }
}
