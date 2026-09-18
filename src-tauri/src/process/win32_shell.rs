use std::io::Cursor;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::LazyLock;
use dashmap::DashMap;
use image::{ImageBuffer, Rgba};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Graphics::Gdi::*;

#[repr(C)]
struct SHFILEINFOW {
    hIcon: HICON,
    iIcon: i32,
    dwAttributes: u32,
    szDisplayName: [u16; 260],
    szTypeName: [u16; 80],
}

const SHGFI_ICON: u32 = 0x000000100;
const SHGFI_LARGEICON: u32 = 0x000000000;

#[link(name = "shell32")]
extern "system" {
    fn SHGetFileInfoW(
        pszPath: *const u16,
        dwFileAttributes: u32,
        psfi: *mut SHFILEINFOW,
        cbFileInfo: u32,
        uFlags: u32,
    ) -> usize;
}

use std::time::{Duration, Instant};

static ICON_CACHE: LazyLock<DashMap<String, String>> = LazyLock::new(|| DashMap::new());
static PID_PATH_CACHE: LazyLock<DashMap<u32, (String, Instant)>> = LazyLock::new(|| DashMap::new());

/// Retrieve the full executable image path for a given PID with TTL-based recycling protection
pub fn get_process_path(pid: u32) -> Option<String> {
    if pid == 0 {
        return Some("System Idle Process".to_string());
    }
    if pid == 4 {
        return Some("ntoskrnl.exe".to_string());
    }

    let now = Instant::now();
    if let Some(cached) = PID_PATH_CACHE.get(&pid) {
        let (ref path, timestamp) = *cached;
        if now.duration_since(timestamp) < Duration::from_secs(5) {
            return Some(path.clone());
        }
    }

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            PID_PATH_CACHE.remove(&pid);
            return None;
        }

        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;

        let success = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size);
        CloseHandle(handle);

        if success != 0 && size > 0 {
            let path_str = String::from_utf16_lossy(&buffer[..size as usize]);
            PID_PATH_CACHE.insert(pid, (path_str.clone(), now));
            Some(path_str)
        } else {
            PID_PATH_CACHE.remove(&pid);
            None
        }
    }
}

/// Extract process base name from full path
pub fn get_process_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// Extract high-resolution Windows icon as Base64-encoded PNG data URI
pub fn extract_icon_base64(exe_path: &str) -> Option<String> {
    if exe_path.is_empty() {
        return None;
    }

    // Check cache first
    if let Some(cached) = ICON_CACHE.get(exe_path) {
        return Some(cached.clone());
    }

    let wide_path: Vec<u16> = std::ffi::OsStr::new(exe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut sh_info: SHFILEINFOW = std::mem::zeroed();
        let ret = SHGetFileInfoW(
            wide_path.as_ptr(),
            0,
            &mut sh_info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );

        if ret == 0 || sh_info.hIcon.is_null() {
            return None;
        }

        let h_icon = sh_info.hIcon;
        let mut icon_info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(h_icon, &mut icon_info) == 0 {
            DestroyIcon(h_icon);
            return None;
        }

        let h_bm_color = icon_info.hbmColor;
        let h_bm_mask = icon_info.hbmMask;

        let h_dc = GetDC(std::ptr::null_mut());
        let mem_dc = CreateCompatibleDC(h_dc);

        let mut bm: BITMAP = std::mem::zeroed();
        GetObjectW(
            h_bm_color as *mut _,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bm as *mut _ as *mut _,
        );

        let width = bm.bmWidth as u32;
        let height = bm.bmHeight as u32;

        if width == 0 || height == 0 {
            if !h_bm_color.is_null() { DeleteObject(h_bm_color as *mut _); }
            if !h_bm_mask.is_null() { DeleteObject(h_bm_mask as *mut _); }
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), h_dc);
            DestroyIcon(h_icon);
            return None;
        }

        let mut bi: BITMAPINFOHEADER = std::mem::zeroed();
        bi.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.biWidth = width as i32;
        bi.biHeight = -(height as i32); // Top-down DIB
        bi.biPlanes = 1;
        bi.biBitCount = 32;
        bi.biCompression = BI_RGB;

        let mut raw_pixels = vec![0u8; (width * height * 4) as usize];

        GetDIBits(
            mem_dc,
            h_bm_color,
            0,
            height,
            raw_pixels.as_mut_ptr() as *mut _,
            &mut bi as *mut _ as *mut _,
            DIB_RGB_COLORS,
        );

        // Cleanup Win32 GDI handles
        if !h_bm_color.is_null() { DeleteObject(h_bm_color as *mut _); }
        if !h_bm_mask.is_null() { DeleteObject(h_bm_mask as *mut _); }
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), h_dc);
        DestroyIcon(h_icon);

        // Convert BGRA to RGBA
        for chunk in raw_pixels.chunks_exact_mut(4) {
            let b = chunk[0];
            let r = chunk[2];
            chunk[0] = r;
            chunk[2] = b;
            if chunk[3] == 0 {
                chunk[3] = 255;
            }
        }

        // Encode to PNG buffer
        if let Some(img) = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, raw_pixels) {
            let mut png_bytes = Vec::new();
            if img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png).is_ok() {
                let base64_str = format!("data:image/png;base64,{}", BASE64.encode(&png_bytes));
                ICON_CACHE.insert(exe_path.to_string(), base64_str.clone());
                return Some(base64_str);
            }
        }
    }

    None
}

/// Open Windows Explorer with the given file selected
pub fn open_file_in_explorer(path_str: &str) -> Result<(), String> {
    let p = Path::new(path_str);
    if !p.exists() {
        return Err(format!("File path does not exist: {}", path_str));
    }

    let status = std::process::Command::new("explorer.exe")
        .arg(format!("/select,\"{}\"", path_str))
        .spawn();

    match status {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to open Explorer: {}", e)),
    }
}

/// Terminate process by PID
pub fn terminate_process_by_pid(pid: u32) -> Result<(), String> {
    if pid <= 4 {
        return Err("Cannot terminate core system process".to_string());
    }

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            return Err(format!("Unable to open process {} for termination (Access Denied)", pid));
        }

        let ret = TerminateProcess(handle, 1);
        CloseHandle(handle);

        if ret != 0 {
            Ok(())
        } else {
            Err(format!("TerminateProcess failed with error: {}", std::io::Error::last_os_error()))
        }
    }
}

/// Check if current process is running with elevated Administrator privileges
pub fn is_running_as_admin() -> bool {
    use windows_sys::Win32::Security::*;

    unsafe {
        let mut handle: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut ret_len = 0;
        let success = GetTokenInformation(
            handle,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );

        CloseHandle(handle);
        success != 0 && elevation.TokenIsElevated != 0
    }
}
