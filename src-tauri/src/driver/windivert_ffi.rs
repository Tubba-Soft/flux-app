use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use libloading::{Library, Symbol};

pub const WINDIVERT_LAYER_NETWORK: u32 = 0;
pub const WINDIVERT_LAYER_FLOW: u32 = 2;

pub const WINDIVERT_FLAG_SNIFF: u64 = 0x0001;
pub const WINDIVERT_FLAG_DROP: u64 = 0x0002;

pub const WINDIVERT_PARAM_QUEUE_LENGTH: u32 = 0;
pub const WINDIVERT_PARAM_QUEUE_TIME: u32 = 1;
pub const WINDIVERT_PARAM_QUEUE_SIZE: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafeWinDivertHandle(pub *mut c_void);

unsafe impl Send for SafeWinDivertHandle {}
unsafe impl Sync for SafeWinDivertHandle {}

pub const INVALID_WINDIVERT_HANDLE: SafeWinDivertHandle = SafeWinDivertHandle(-1isize as *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WinDivertAddress {
    pub timestamp: i64,
    pub bitfields: u32,
    pub reserved2: u32,
    pub data: [u8; 64],
}

impl WinDivertAddress {
    pub fn new() -> Self {
        Self {
            timestamp: 0,
            bitfields: 0,
            reserved2: 0,
            data: [0u8; 64],
        }
    }

    #[inline]
    pub fn is_outbound(&self) -> bool {
        (self.bitfields & (1 << 17)) != 0
    }

    #[inline]
    pub fn is_inbound(&self) -> bool {
        !self.is_outbound()
    }

    #[inline]
    pub fn is_loopback(&self) -> bool {
        (self.bitfields & (1 << 18)) != 0
    }

    #[inline]
    pub fn is_ipv6(&self) -> bool {
        (self.bitfields & (1 << 20)) != 0
    }
}

type FnOpen = unsafe extern "system" fn(
    filter: *const c_char,
    layer: u32,
    priority: i16,
    flags: u64,
) -> *mut c_void;

type FnRecv = unsafe extern "system" fn(
    handle: *mut c_void,
    packet: *mut c_void,
    packet_len: u32,
    recv_len: *mut u32,
    addr: *mut WinDivertAddress,
) -> c_int;

type FnSend = unsafe extern "system" fn(
    handle: *mut c_void,
    packet: *const c_void,
    packet_len: u32,
    send_len: *mut u32,
    addr: *const WinDivertAddress,
) -> c_int;

type FnSetParam = unsafe extern "system" fn(
    handle: *mut c_void,
    param: u32,
    val: u64,
) -> c_int;

type FnClose = unsafe extern "system" fn(handle: *mut c_void) -> c_int;

type FnCalcChecksums = unsafe extern "system" fn(
    packet: *mut c_void,
    packet_len: u32,
    addr: *mut WinDivertAddress,
    flags: u64,
) -> c_int;

#[derive(Clone)]
pub struct WinDivertLib {
    _lib: Arc<Library>,
    fn_open: FnOpen,
    fn_recv: FnRecv,
    fn_send: FnSend,
    fn_set_param: FnSetParam,
    fn_close: FnClose,
    fn_calc_checksums: FnCalcChecksums,
}

unsafe impl Send for WinDivertLib {}
unsafe impl Sync for WinDivertLib {}

impl WinDivertLib {
    pub fn load() -> Result<Self, String> {
        let possible_paths = [
            PathBuf::from("WinDivert.dll"),
            PathBuf::from("bin/WinDivert.dll"),
            PathBuf::from("src-tauri/bin/WinDivert.dll"),
        ];

        let mut loaded_lib: Option<Library> = None;
        let mut last_err = String::new();

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let dll_in_exe = exe_dir.join("WinDivert.dll");
                if dll_in_exe.exists() {
                    match unsafe { Library::new(&dll_in_exe) } {
                        Ok(lib) => loaded_lib = Some(lib),
                        Err(e) => last_err = format!("Failed to load from exe dir: {}", e),
                    }
                }
            }
        }

        if loaded_lib.is_none() {
            for p in &possible_paths {
                if p.exists() {
                    match unsafe { Library::new(p) } {
                        Ok(lib) => {
                            loaded_lib = Some(lib);
                            break;
                        }
                        Err(e) => last_err = format!("Failed loading {:?}: {}", p, e),
                    }
                }
            }
        }

        if loaded_lib.is_none() {
            match unsafe { Library::new("WinDivert.dll") } {
                Ok(lib) => loaded_lib = Some(lib),
                Err(e) => return Err(format!("WinDivert.dll not found or cannot be loaded: {}. Last error: {}", e, last_err)),
            }
        }

        let lib = loaded_lib.unwrap();

        unsafe {
            let fn_open: Symbol<FnOpen> = lib.get(b"WinDivertOpen\0")
                .map_err(|e| format!("Missing WinDivertOpen: {}", e))?;
            let fn_recv: Symbol<FnRecv> = lib.get(b"WinDivertRecv\0")
                .map_err(|e| format!("Missing WinDivertRecv: {}", e))?;
            let fn_send: Symbol<FnSend> = lib.get(b"WinDivertSend\0")
                .map_err(|e| format!("Missing WinDivertSend: {}", e))?;
            let fn_set_param: Symbol<FnSetParam> = lib.get(b"WinDivertSetParam\0")
                .map_err(|e| format!("Missing WinDivertSetParam: {}", e))?;
            let fn_close: Symbol<FnClose> = lib.get(b"WinDivertClose\0")
                .map_err(|e| format!("Missing WinDivertClose: {}", e))?;
            let fn_calc_checksums: Symbol<FnCalcChecksums> = lib.get(b"WinDivertHelperCalcChecksums\0")
                .map_err(|e| format!("Missing WinDivertHelperCalcChecksums: {}", e))?;

            Ok(Self {
                fn_open: *fn_open,
                fn_recv: *fn_recv,
                fn_send: *fn_send,
                fn_set_param: *fn_set_param,
                fn_close: *fn_close,
                fn_calc_checksums: *fn_calc_checksums,
                _lib: Arc::new(lib),
            })
        }
    }

    pub fn open(&self, filter: &str, layer: u32, priority: i16, flags: u64) -> Result<SafeWinDivertHandle, String> {
        let c_filter = CString::new(filter).map_err(|e| e.to_string())?;
        let raw = unsafe { (self.fn_open)(c_filter.as_ptr(), layer, priority, flags) };
        if raw == INVALID_WINDIVERT_HANDLE.0 || raw.is_null() {
            let err = std::io::Error::last_os_error();
            Err(format!("WinDivertOpen failed (OS Error: {} - {})", err.raw_os_error().unwrap_or(0), err))
        } else {
            Ok(SafeWinDivertHandle(raw))
        }
    }

    pub fn set_param(&self, handle: SafeWinDivertHandle, param: u32, value: u64) -> bool {
        let res = unsafe { (self.fn_set_param)(handle.0, param, value) };
        res != 0
    }

    pub fn recv(&self, handle: SafeWinDivertHandle, buf: &mut [u8], addr: &mut WinDivertAddress) -> Result<usize, std::io::Error> {
        let mut recv_len: u32 = 0;
        let res = unsafe {
            (self.fn_recv)(
                handle.0,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as u32,
                &mut recv_len,
                addr as *mut WinDivertAddress,
            )
        };
        if res != 0 {
            Ok(recv_len as usize)
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    pub fn send(&self, handle: SafeWinDivertHandle, buf: &[u8], addr: &WinDivertAddress) -> Result<usize, std::io::Error> {
        let mut send_len: u32 = 0;
        let res = unsafe {
            (self.fn_send)(
                handle.0,
                buf.as_ptr() as *const c_void,
                buf.len() as u32,
                &mut send_len,
                addr as *const WinDivertAddress,
            )
        };
        if res != 0 {
            Ok(send_len as usize)
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    pub fn calc_checksums(&self, buf: &mut [u8], addr: &mut WinDivertAddress) {
        unsafe {
            (self.fn_calc_checksums)(
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as u32,
                addr as *mut WinDivertAddress,
                0,
            );
        }
    }

    pub fn close(&self, handle: SafeWinDivertHandle) {
        if handle != INVALID_WINDIVERT_HANDLE && !handle.0.is_null() {
            unsafe {
                (self.fn_close)(handle.0);
            }
        }
    }
}
