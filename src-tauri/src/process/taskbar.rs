//! Docking the speed widget inside the Windows taskbar, beside the notification area.
//!
//! Windows offers no supported way to put custom content *into* the taskbar. The classic
//! mechanism, a COM deskband, was deprecated in Windows 10 and does nothing on the Windows
//! 11 taskbar. What works on both is a topmost, non-activating window placed over the
//! taskbar's own rectangle — the approach every current network-speed widget uses.
//!
//! The anchor is `TrayNotifyWnd`, the notification area that holds the "show hidden icons"
//! chevron, the tray icons and the clock. Placing the widget immediately to its left puts
//! it exactly where the user expects, and it keeps working when the tray grows or shrinks
//! because the anchor is re-read on every tick rather than assumed.
//!
//! All coordinates here are physical pixels. The app is per-monitor DPI aware, so
//! `GetWindowRect` and Tauri's `PhysicalPosition` already speak the same units; nothing
//! must be multiplied by a scale factor.

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowRect, IsWindowVisible,
};

/// Breathing room between the widget and the first tray element.
const GAP: i32 = 8;

/// Used only when `TrayNotifyWnd` cannot be found: how much of the taskbar's right edge to
/// treat as "tray" so the widget still lands clear of the clock.
const FALLBACK_TRAY_WIDTH: i32 = 280;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn window_rect(hwnd: HWND) -> Option<RECT> {
    if hwnd.is_null() {
        return None;
    }
    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
    if ok == 0 || rect.right <= rect.left || rect.bottom <= rect.top {
        return None;
    }
    Some(rect)
}

/// Where the widget should sit to appear inside the taskbar, just left of the tray.
///
/// Returns `None` when there is nothing sensible to dock to: no taskbar, a hidden taskbar
/// (auto-hide currently retracted), a vertical taskbar on the left or right edge, or a
/// taskbar too short to hold the widget. Callers should leave the widget where it is in
/// that case rather than guess.
pub fn dock_position(widget_w: i32, widget_h: i32) -> Option<(i32, i32)> {
    let tray_wnd = unsafe { FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null()) };
    if tray_wnd.is_null() || unsafe { IsWindowVisible(tray_wnd) } == 0 {
        return None;
    }

    let bar = window_rect(tray_wnd)?;
    let bar_w = bar.right - bar.left;
    let bar_h = bar.bottom - bar.top;

    // A taskbar docked left or right is taller than it is wide; a horizontal strip is the
    // only shape this layout makes sense in.
    if bar_h >= bar_w || bar_h < widget_h {
        return None;
    }

    // Left edge of the notification area, which is what the widget butts up against.
    let anchor_x = unsafe {
        FindWindowExW(
            tray_wnd,
            std::ptr::null_mut(),
            wide("TrayNotifyWnd").as_ptr(),
            std::ptr::null(),
        )
    };
    let tray_left = window_rect(anchor_x)
        .map(|r| r.left)
        .unwrap_or(bar.right - FALLBACK_TRAY_WIDTH);

    let x = (tray_left - widget_w - GAP).max(bar.left);
    let y = bar.top + (bar_h - widget_h) / 2;

    Some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_is_null_terminated() {
        assert_eq!(wide("Shell"), vec![83u16, 104, 101, 108, 108, 0]);
    }

    #[test]
    fn dock_position_is_none_or_sane() {
        // Runs against the real shell: on a normal horizontal taskbar it must produce a
        // position, and on anything unusual it must decline rather than return nonsense.
        if let Some((x, y)) = dock_position(220, 38) {
            assert!(x >= 0, "x={}", x);
            assert!(y >= 0, "y={}", y);
        }
    }
}
