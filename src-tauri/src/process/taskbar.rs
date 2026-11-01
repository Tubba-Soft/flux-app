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
    FindWindowExW, FindWindowW, GetWindowRect, IsWindowVisible, SetWindowPos, HWND_TOPMOST,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
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

/// The taskbar strip and the left edge of its notification area.
///
/// `None` when there is nothing sensible to dock to: no taskbar, a hidden one (auto-hide
/// currently retracted), a vertical taskbar on the left or right edge, or one too short to
/// hold the widget. Callers should leave the widget alone in that case rather than guess.
fn taskbar_geometry(widget_h: i32) -> Option<(RECT, i32)> {
    let tray_wnd = unsafe { FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null()) };
    if tray_wnd.is_null() || unsafe { IsWindowVisible(tray_wnd) } == 0 {
        return None;
    }

    let bar = window_rect(tray_wnd)?;

    // A taskbar docked left or right is taller than it is wide; a horizontal strip is the
    // only shape this layout makes sense in.
    if (bar.bottom - bar.top) >= (bar.right - bar.left) || (bar.bottom - bar.top) < widget_h {
        return None;
    }

    // Left edge of the notification area, which is what the widget butts up against.
    let anchor = unsafe {
        FindWindowExW(
            tray_wnd,
            std::ptr::null_mut(),
            wide("TrayNotifyWnd").as_ptr(),
            std::ptr::null(),
        )
    };
    let tray_left = window_rect(anchor)
        .map(|r| r.left)
        .unwrap_or(bar.right - FALLBACK_TRAY_WIDTH);

    Some((bar, tray_left))
}

/// Where the widget should sit inside the taskbar.
///
/// With no `offset_from_left` it hugs the notification area, so it keeps its place beside
/// the hidden-icons chevron as tray icons come and go. Once the user has dragged it the
/// offset pins it to a spot measured from the taskbar's left edge instead, which is what
/// makes a deliberate placement survive the tray resizing. Either way it is clamped clear
/// of the tray and vertically centred - the strip is too short for vertical freedom to
/// mean anything.
pub fn dock_position(widget_w: i32, widget_h: i32, offset_from_left: Option<i32>) -> Option<(i32, i32)> {
    let (bar, tray_left) = taskbar_geometry(widget_h)?;

    let rightmost = (tray_left - widget_w - GAP).max(bar.left);
    let x = match offset_from_left {
        Some(off) => (bar.left + off).clamp(bar.left, rightmost),
        None => rightmost,
    };
    let y = bar.top + ((bar.bottom - bar.top) - widget_h) / 2;

    Some((x, y))
}

/// Offset to store when the widget is dropped at `(x, y)`, or `None` if it was dropped
/// off the taskbar and should undock.
///
/// The test is the widget's vertical midpoint: dropped over the strip it stays docked and
/// slides to where it was put, dropped above it the user clearly wants it on the desktop.
pub fn dock_offset_for(x: i32, y: i32, widget_w: i32, widget_h: i32) -> Option<i32> {
    let (bar, tray_left) = taskbar_geometry(widget_h)?;

    let midpoint_y = y + widget_h / 2;
    if midpoint_y < bar.top || midpoint_y > bar.bottom {
        return None;
    }
    if x + widget_w < bar.left || x > bar.right {
        return None;
    }

    let rightmost = (tray_left - widget_w - GAP).max(bar.left);
    Some(x.clamp(bar.left, rightmost) - bar.left)
}

/// Lift the window back to the front of the topmost band.
///
/// The taskbar is topmost too, and within that band whichever window was raised last wins.
/// Clicking the taskbar raises it over the widget, so simply having set always-on-top once
/// is not enough: the claim has to be renewed. `SWP_NOACTIVATE` keeps focus where it is, so
/// renewing it never steals the user's keyboard focus or dismisses a menu.
pub fn raise_topmost(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
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
        if let Some((x, y)) = dock_position(220, 38, None) {
            assert!(x >= 0, "x={}", x);
            assert!(y >= 0, "y={}", y);
        }
    }

    #[test]
    fn a_dragged_offset_moves_the_widget_but_never_over_the_tray() {
        let Some((default_x, default_y)) = dock_position(220, 38, None) else {
            return; // no horizontal taskbar on this machine
        };

        // A small offset lands the widget to the left of its default spot.
        let (left_x, left_y) = dock_position(220, 38, Some(100)).unwrap();
        assert!(left_x <= default_x, "{} should be left of {}", left_x, default_x);
        assert_eq!(left_y, default_y, "vertical placement must not change");

        // An absurd offset is clamped back to the rightmost legal spot, never onto the tray.
        let (far_x, _) = dock_position(220, 38, Some(1_000_000)).unwrap();
        assert_eq!(far_x, default_x);
    }

    #[test]
    fn a_drop_above_the_taskbar_undocks() {
        if dock_position(220, 38, None).is_none() {
            return;
        }
        // Far above any taskbar: must not produce an offset.
        assert_eq!(dock_offset_for(400, 10, 220, 38), None);
    }

    #[test]
    fn a_drop_on_the_taskbar_round_trips() {
        let Some((x, y)) = dock_position(220, 38, Some(300)) else {
            return;
        };
        let offset = dock_offset_for(x, y, 220, 38).expect("drop on the strip must dock");
        let (again_x, again_y) = dock_position(220, 38, Some(offset)).unwrap();
        assert_eq!((again_x, again_y), (x, y), "re-docking must be stable");
    }
}
