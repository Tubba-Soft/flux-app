#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused, non_snake_case, dead_code)]

mod models;
mod driver;
mod shaper;
mod net;
mod process;
mod storage;
mod alerts;

use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use parking_lot::RwLock;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use log::{info, warn, error};

use crate::models::{AppRule, DriverStatus, GlobalTelemetry, NewAppAlert, ProcessTraffic, StreamRule, StreamTraffic, WidgetConfig};
use crate::driver::packet_engine::PacketEngine;
use crate::net::ip_helper::IpHelperResolver;
use crate::process::win32_shell::{
    extract_icon_base64, get_process_name_from_path, get_process_path, is_running_as_admin,
    open_file_in_explorer, terminate_process_by_pid,
};
use crate::shaper::token_bucket::TokenBucketShaper;
use crate::storage::db::Database;
use crate::alerts::app_detector::AppDetector;

#[derive(Clone)]
pub struct AppState {
    pub ip_resolver: Arc<IpHelperResolver>,
    pub shaper: Arc<TokenBucketShaper>,
    pub db: Arc<Database>,
    pub detector: Arc<AppDetector>,
    pub packet_engine: Arc<PacketEngine>,
    pub driver_status: Arc<RwLock<DriverStatus>>,
    pub app_rules_cache: Arc<DashMap<String, AppRule>>,
    pub stream_rules_cache: Arc<DashMap<String, StreamRule>>,
    pub show_taskbar_widget: Arc<AtomicBool>,
}

#[tauri::command]
fn get_driver_status(state: State<AppState>) -> DriverStatus {
    let mut status = state.driver_status.read().clone();
    status.packet_count = state.packet_engine.packet_counter.load(Ordering::Relaxed);
    status.diverted_bytes = state.packet_engine.byte_counter.load(Ordering::Relaxed);
    status
}

#[tauri::command]
fn get_telemetry_snapshot(state: State<AppState>) -> Vec<ProcessTraffic> {
    build_telemetry_tree(&state)
}

#[tauri::command]
fn set_process_rule(state: State<AppState>, rule: AppRule) -> Result<(), String> {
    // 1. Update shaper engine
    state.shaper.update_process_rule(
        &rule.path,
        rule.is_blocked,
        rule.down_limit_kbps,
        rule.up_limit_kbps,
    );

    // 2. Update fast in-memory cache
    state.app_rules_cache.insert(rule.path.clone(), rule.clone());

    // 3. Persist to SQLite
    state.db.set_app_rule(&rule)?;
    Ok(())
}

#[tauri::command]
fn set_stream_rule(state: State<AppState>, rule: StreamRule) -> Result<(), String> {
    // 1. Update shaper engine
    state.shaper.update_stream_rule(
        &rule.stream_id,
        rule.is_blocked,
        rule.down_limit_kbps,
        rule.up_limit_kbps,
    );

    // 2. Update fast in-memory cache
    state.stream_rules_cache.insert(rule.stream_id.clone(), rule.clone());

    // 3. Persist to SQLite
    state.db.set_stream_rule(&rule)?;
    Ok(())
}

#[tauri::command]
fn open_file_location(path: String) -> Result<(), String> {
    open_file_in_explorer(&path)
}

#[tauri::command]
fn terminate_process(pid: u32) -> Result<(), String> {
    terminate_process_by_pid(pid)
}

#[tauri::command]
fn close_stream_socket(
    state: State<AppState>,
    local_ip: String,
    local_port: u16,
    remote_ip: String,
    remote_port: u16,
) -> Result<(), String> {
    let lip = Ipv4Addr::from_str(&local_ip).map_err(|e| e.to_string())?;
    let rip = Ipv4Addr::from_str(&remote_ip).map_err(|e| e.to_string())?;
    state.ip_resolver.close_tcp_socket(lip, local_port, rip, remote_port)
}

#[tauri::command]
fn request_admin_elevation() -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = std::process::Command::new("powershell")
        .arg("-Command")
        .arg(format!("Start-Process -FilePath \"{}\" -Verb RunAs", current_exe.to_string_lossy()))
        .spawn();

    match status {
        Ok(_) => {
            std::process::exit(0);
        }
        Err(e) => Err(format!("Failed to elevate privileges: {}", e)),
    }
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Only http:// and https:// URLs are allowed".to_string());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn enforce_remote_lockdown(app_handle: AppHandle, state: State<AppState>, reason: String) -> Result<(), String> {
    warn!("[RemoteLockdown] Enforcing kill-switch lockdown: {}", reason);
    // 1. Immediately stop WinDivert packet engine to release network and ensure user has normal internet
    let _ = state.packet_engine.stop();

    // 2. Mark driver status as remote_lockdown
    {
        let mut status = state.driver_status.write();
        status.is_driver_loaded = false;
        status.mode = "remote_lockdown".to_string();
        status.error_message = Some(format!("Remote lockdown active: {}", reason));
    }

    // 3. Hide widget window if active
    if let Some(widget_win) = app_handle.get_webview_window("widget") {
        let _ = widget_win.hide();
    }

    // 4. Show and focus main window so user sees the notification immediately
    if let Some(main_win) = app_handle.get_webview_window("main") {
        let _ = main_win.show();
        let _ = main_win.unminimize();
        let _ = main_win.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn release_remote_lockdown(app_handle: AppHandle, state: State<AppState>) -> Result<(), String> {
    info!("[RemoteLockdown] Releasing kill-switch lockdown - restoring engine");
    {
        let mut status = state.driver_status.write();
        if status.mode == "remote_lockdown" {
            status.mode = "passive_monitor".to_string();
            status.error_message = None;
        }
    }
    // Restore widget window if it was enabled
    if state.show_taskbar_widget.load(Ordering::Relaxed) {
        if let Some(widget_win) = app_handle.get_webview_window("widget") {
            let _ = widget_win.show();
        }
    }
    Ok(())
}

#[tauri::command]
fn force_exit_app() {
    std::process::exit(0);
}

fn format_speed_label(bps: u64) -> String {
    let bytes_per_sec = bps as f64;
    if bytes_per_sec >= 1_048_576.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.0} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

#[tauri::command]
fn get_autostart() -> bool {
    let output = std::process::Command::new("schtasks")
        .args(["/query", "/tn", "Flux_Autostart"])
        .output();
    match output {
        Ok(out) if out.status.success() => true,
        _ => {
            // Check legacy task name
            let legacy = std::process::Command::new("schtasks")
                .args(["/query", "/tn", "NetFlowStudio_Autostart"])
                .output();
            matches!(legacy, Ok(out) if out.status.success())
        }
    }
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        let installed_exe = std::path::PathBuf::from(r"C:\Program Files\Flux\flux.exe");
        let (exe_path, work_dir) = if installed_exe.exists() {
            (installed_exe.to_string_lossy().to_string(), r"C:\Program Files\Flux".to_string())
        } else {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let work = exe.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            (exe.to_string_lossy().to_string(), work)
        };

        // Use PowerShell Register-ScheduledTask for a robust autostart with admin + minimized + working directory
        let ps_script = format!(
            r#"
            try {{
                $exe = '{}'
                $dir = '{}'
                $action = New-ScheduledTaskAction -Execute $exe -Argument '--minimized' -WorkingDirectory $dir
                $trigger = New-ScheduledTaskTrigger -AtLogOn
                $principal = New-ScheduledTaskPrincipal -UserId (whoami) -RunLevel Highest -LogonType Interactive
                $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -ExecutionTimeLimit ([TimeSpan]::Zero)
                Unregister-ScheduledTask -TaskName 'Flux_Autostart' -Confirm:$false -ErrorAction SilentlyContinue
                Unregister-ScheduledTask -TaskName 'NetFlowStudio_Autostart' -Confirm:$false -ErrorAction SilentlyContinue
                Register-ScheduledTask -TaskName 'Flux_Autostart' -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Description 'Flux - Auto-start minimized with admin privileges' -Force
                exit 0
            }} catch {{
                exit 1
            }}
            "#,
            exe_path.replace('\'', "''"),
            work_dir.replace('\'', "''")
        );

        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script])
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            info!("[Autostart] Successfully registered scheduled task for {}", exe_path);
            Ok(())
        } else {
            // Fallback to schtasks if PowerShell method fails
            let tr_val = format!(r#"\"\"{}\"\" --minimized"#, exe_path);
            let fallback = std::process::Command::new("schtasks")
                .args(["/create", "/tn", "Flux_Autostart", "/tr", &tr_val, "/sc", "onlogon", "/rl", "highest", "/f"])
                .status()
                .map_err(|e| e.to_string())?;

            if fallback.success() {
                info!("[Autostart] Registered scheduled task via schtasks fallback");
                Ok(())
            } else {
                Err("Failed to register autostart task in Windows Task Scheduler".to_string())
            }
        }
    } else {
        let _ = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", "Flux_Autostart", "/f"])
            .status();
        let _ = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", "NetFlowStudio_Autostart", "/f"])
            .status();
        info!("[Autostart] Removed autostart task");
        Ok(())
    }
}

#[tauri::command]
fn get_run_in_background(state: State<AppState>) -> bool {
    state.db.get_setting("run_in_background", "true") == "true"
}

#[tauri::command]
fn set_run_in_background(state: State<AppState>, enabled: bool) -> Result<(), String> {
    state.db.set_setting("run_in_background", if enabled { "true" } else { "false" })
}

#[tauri::command]
fn get_taskbar_widget(state: State<AppState>) -> bool {
    state.db.get_setting("taskbar_widget", "true") == "true"
}

fn position_widget_window(widget: &tauri::WebviewWindow, db: &Database) {
    if let Ok(Some(monitor)) = widget.primary_monitor() {
        let screen_size = monitor.size();
        let scale = monitor.scale_factor();
        let saved_x = db.get_setting("widget_x", "");
        let saved_y = db.get_setting("widget_y", "");

        let widget_w = (220.0 * scale) as i32;
        let widget_h = (38.0 * scale) as i32;
        let taskbar_h = (48.0 * scale) as i32;
        let margin_right = (16.0 * scale) as i32;
        let margin_bottom = (8.0 * scale) as i32;

        let max_x = ((screen_size.width as i32) - widget_w - 4).max(0);
        let max_y = ((screen_size.height as i32) - taskbar_h - widget_h - 4).max(0);

        let (pos_x, pos_y) = if let (Ok(x), Ok(y)) = (saved_x.parse::<i32>(), saved_y.parse::<i32>()) {
            // Ensure saved position is strictly within currently active monitor bounds
            if x >= 4 && y >= 4 && x <= max_x && y <= max_y {
                (x, y)
            } else {
                let px = (screen_size.width as i32) - widget_w - margin_right;
                let py = (screen_size.height as i32) - taskbar_h - widget_h - margin_bottom;
                (px, py)
            }
        } else {
            // Position cleanly in the bottom right corner, docked ABOVE the taskbar (never covers taskbar icons!)
            let px = (screen_size.width as i32) - widget_w - margin_right;
            let py = (screen_size.height as i32) - taskbar_h - widget_h - margin_bottom;
            (px, py)
        };

        let _ = widget.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(pos_x, pos_y)));
        let _ = widget.set_always_on_top(true);
    }
}

#[tauri::command]
fn save_widget_position(state: State<AppState>, x: i32, y: i32) -> Result<(), String> {
    // Strict safety guard: reject negative coordinates (like -32000 on lock/sleep) or absurd numbers
    if x >= 0 && y >= 0 && x < 15000 && y < 15000 {
        state.db.set_setting("widget_x", &x.to_string())?;
        state.db.set_setting("widget_y", &y.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn reset_widget_position(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let _ = state.db.set_setting("widget_x", "");
    let _ = state.db.set_setting("widget_y", "");
    let _ = state.db.set_setting("taskbar_widget", "true");
    if let Some(widget) = app.get_webview_window("widget") {
        position_widget_window(&widget, &state.db);
        let _ = widget.show();
        let _ = widget.unminimize();
        let _ = widget.set_always_on_top(true);
    }
    Ok(())
}

#[tauri::command]
fn get_widget_config(state: State<AppState>) -> WidgetConfig {
    WidgetConfig {
        locked: state.db.get_setting("widget_locked", "false") == "true",
        auto_transparency: state.db.get_setting("widget_auto_transparency", "true") == "true",
        transparency_delay_secs: state.db.get_setting("widget_transparency_delay", "2").parse().unwrap_or(2),
        transparency_opacity: state.db.get_setting("widget_transparency_opacity", "30").parse().unwrap_or(30),
        color_theme: state.db.get_setting("widget_color_theme", "emerald_cyan"),
        preset: state.db.get_setting("widget_preset", "bottom-right"),
    }
}

#[tauri::command]
fn save_widget_config(app: AppHandle, state: State<AppState>, config: WidgetConfig) -> Result<(), String> {
    state.db.set_setting("widget_locked", if config.locked { "true" } else { "false" })?;
    state.db.set_setting("widget_auto_transparency", if config.auto_transparency { "true" } else { "false" })?;
    state.db.set_setting("widget_transparency_delay", &config.transparency_delay_secs.to_string())?;
    state.db.set_setting("widget_transparency_opacity", &config.transparency_opacity.to_string())?;
    state.db.set_setting("widget_color_theme", &config.color_theme)?;
    state.db.set_setting("widget_preset", &config.preset)?;
    let _ = app.emit("widget-config-changed", &config);
    Ok(())
}

#[tauri::command]
fn start_widget_drag(app: AppHandle) -> Result<(), String> {
    if let Some(widget) = app.get_webview_window("widget") {
        widget.start_dragging().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn set_widget_preset(app: AppHandle, state: State<AppState>, preset: String) -> Result<(), String> {
    if let Some(widget) = app.get_webview_window("widget") {
        if let Ok(Some(monitor)) = widget.primary_monitor() {
            let screen_size = monitor.size();
            let scale = monitor.scale_factor();
            let widget_w = (220.0 * scale) as i32;
            let widget_h = (38.0 * scale) as i32;
            let taskbar_h = (48.0 * scale) as i32;
            let margin = (16.0 * scale) as i32;

            let (px, py) = match preset.as_str() {
                "top-left" => (margin, margin),
                "top-center" => (((screen_size.width as i32) - widget_w) / 2, margin),
                "top-right" => ((screen_size.width as i32) - widget_w - margin, margin),
                "bottom-left" => (margin, (screen_size.height as i32) - taskbar_h - widget_h - (8.0 * scale) as i32),
                "bottom-center" => (((screen_size.width as i32) - widget_w) / 2, (screen_size.height as i32) - taskbar_h - widget_h - (8.0 * scale) as i32),
                _ => ((screen_size.width as i32) - widget_w - margin, (screen_size.height as i32) - taskbar_h - widget_h - (8.0 * scale) as i32),
            };

            let _ = widget.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(px, py)));
            let _ = widget.set_always_on_top(true);
            state.db.set_setting("widget_x", &px.to_string())?;
            state.db.set_setting("widget_y", &py.to_string())?;
            state.db.set_setting("widget_preset", &preset)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn toggle_taskbar_widget(app: AppHandle, state: State<AppState>, enabled: bool) -> Result<(), String> {
    state.show_taskbar_widget.store(enabled, Ordering::Relaxed);
    state.db.set_setting("taskbar_widget", if enabled { "true" } else { "false" })?;
    if let Some(widget) = app.get_webview_window("widget") {
        if enabled {
            position_widget_window(&widget, &state.db);
            let _ = widget.show();
            let _ = widget.set_always_on_top(true);
        } else {
            let _ = widget.hide();
        }
    }
    Ok(())
}

#[tauri::command]
fn get_global_telemetry(state: State<AppState>) -> GlobalTelemetry {
    let (d_spd, u_spd, tot_d, tot_u) = state.packet_engine.get_global_speeds();
    let (today_d, today_u) = state.db.get_today_traffic();
    GlobalTelemetry {
        down_speed_bps: d_spd,
        up_speed_bps: u_spd,
        total_down_bytes: tot_d,
        total_up_bytes: tot_u,
        today_down_bytes: today_d,
        today_up_bytes: today_u,
    }
}

/// Helper to aggregate active IP sockets and packet engine metrics into hierarchical tree
fn build_telemetry_tree(state: &AppState) -> Vec<ProcessTraffic> {
    use std::collections::HashMap;

    let mut process_map: HashMap<u32, ProcessTraffic> = HashMap::new();

    // Fast in-memory lookup for rules (Zero SQLite disk contention)
    let rules_by_path = &state.app_rules_cache;
    let rules_by_stream = &state.stream_rules_cache;

    // Helper closure to ensure ProcessTraffic exists
    let mut ensure_proc = |proc_map: &mut HashMap<u32, ProcessTraffic>, pid: u32, known_path: Option<&str>, known_name: Option<&str>| {
        if !proc_map.contains_key(&pid) {
            let path = if pid == 0 {
                "System".to_string()
            } else if pid == 4 {
                "C:\\Windows\\System32\\ntoskrnl.exe".to_string()
            } else if let Some(p) = known_path {
                p.to_string()
            } else {
                get_process_path(pid).unwrap_or_else(|| "Unknown".to_string())
            };

            let name = if pid == 0 {
                "System Idle / Sockets".to_string()
            } else if pid == 4 {
                "System Kernel".to_string()
            } else if let Some(n) = known_name {
                n.to_string()
            } else {
                get_process_name_from_path(&path)
            };

            let icon = extract_icon_base64(&path);
            let (is_blocked, down_limit, up_limit, priority) = if let Some(rule) = rules_by_path.get(&path) {
                (rule.is_blocked, rule.down_limit_kbps, rule.up_limit_kbps, rule.priority.clone())
            } else {
                (false, None, None, "Normal".to_string())
            };

            let proc_stat = state.packet_engine.processes.get(&pid);
            let p_down = proc_stat.as_ref().map(|st| st.down_speed_bps.load(Ordering::Relaxed)).unwrap_or(0);
            let p_up = proc_stat.as_ref().map(|st| st.up_speed_bps.load(Ordering::Relaxed)).unwrap_or(0);
            let p_tot_down = proc_stat.as_ref().map(|st| st.total_down_bytes.load(Ordering::Relaxed)).unwrap_or(0);
            let p_tot_up = proc_stat.as_ref().map(|st| st.total_up_bytes.load(Ordering::Relaxed)).unwrap_or(0);

            proc_map.insert(
                pid,
                ProcessTraffic {
                    pid,
                    name,
                    path,
                    icon_base64: icon,
                    down_speed_bps: p_down,
                    up_speed_bps: p_up,
                    total_down_bytes: p_tot_down,
                    total_up_bytes: p_tot_up,
                    active_streams_count: 0,
                    is_blocked,
                    down_limit_kbps: down_limit,
                    up_limit_kbps: up_limit,
                    priority,
                    streams: Vec::new(),
                },
            );
        }
    };

    // 1. First populate all processes tracked by packet_engine (guaranteed real live traffic)
    for entry in state.packet_engine.processes.iter() {
        let pid = *entry.key();
        let stat = entry.value();
        ensure_proc(&mut process_map, pid, Some(&stat.path), Some(&stat.name));
        if let Some(p) = process_map.get_mut(&pid) {
            p.down_speed_bps = stat.down_speed_bps.load(Ordering::Relaxed);
            p.up_speed_bps = stat.up_speed_bps.load(Ordering::Relaxed);
            p.total_down_bytes = stat.total_down_bytes.load(Ordering::Relaxed);
            p.total_up_bytes = stat.total_up_bytes.load(Ordering::Relaxed);
        }
    }

    // 2. Attach live active streams directly from packet_engine.streams
    let mut added_stream_ids: HashMap<u32, std::collections::HashSet<String>> = HashMap::new();

    for entry in state.packet_engine.streams.iter() {
        let s = entry.value();
        let pid = s.pid;
        let (stream_is_blocked, stream_down_limit, stream_up_limit) = if let Some(s_rule) = rules_by_stream.get(&s.stream_id) {
            (s_rule.is_blocked, s_rule.down_limit_kbps, s_rule.up_limit_kbps)
        } else {
            (false, None, None)
        };

        let stream_traffic = StreamTraffic {
            id: s.stream_id.clone(),
            protocol: s.protocol.clone(),
            local_ip: s.local_ip.to_string(),
            local_port: s.local_port,
            remote_ip: s.remote_ip.to_string(),
            remote_port: s.remote_port,
            remote_domain: s.remote_domain.clone(),
            down_speed_bps: s.down_speed_bps.load(Ordering::Relaxed),
            up_speed_bps: s.up_speed_bps.load(Ordering::Relaxed),
            total_down_bytes: s.total_down_bytes.load(Ordering::Relaxed),
            total_up_bytes: s.total_up_bytes.load(Ordering::Relaxed),
            is_blocked: stream_is_blocked,
            down_limit_kbps: stream_down_limit,
            up_limit_kbps: stream_up_limit,
            state: "ACTIVE".to_string(),
        };

        ensure_proc(&mut process_map, pid, s.process_path.as_deref(), None);
        if let Some(p) = process_map.get_mut(&pid) {
            p.streams.push(stream_traffic);
        }
        added_stream_ids.entry(pid).or_default().insert(s.stream_id.clone());
    }

    // 3. Supplement with active IP Helper sockets (for sockets that are idle or listening)
    let active_sockets = state.ip_resolver.get_active_sockets();
    for s in &active_sockets {
        let stream_id = format!("{}-{}:{}-{}:{}", s.protocol, s.local_ip, s.local_port, s.remote_ip, s.remote_port);
        let pid = s.pid;

        let already_added = added_stream_ids.get(&pid).map(|set| set.contains(&stream_id)).unwrap_or(false);
        if !already_added {
            let (stream_is_blocked, stream_down_limit, stream_up_limit) = if let Some(s_rule) = rules_by_stream.get(&stream_id) {
                (s_rule.is_blocked, s_rule.down_limit_kbps, s_rule.up_limit_kbps)
            } else {
                (false, None, None)
            };

            let stream_traffic = StreamTraffic {
                id: stream_id,
                protocol: s.protocol.clone(),
                local_ip: s.local_ip.to_string(),
                local_port: s.local_port,
                remote_ip: s.remote_ip.to_string(),
                remote_port: s.remote_port,
                remote_domain: None,
                down_speed_bps: 0,
                up_speed_bps: 0,
                total_down_bytes: 0,
                total_up_bytes: 0,
                is_blocked: stream_is_blocked,
                down_limit_kbps: stream_down_limit,
                up_limit_kbps: stream_up_limit,
                state: s.state.clone(),
            };

            ensure_proc(&mut process_map, pid, None, None);
            if let Some(p) = process_map.get_mut(&pid) {
                p.streams.push(stream_traffic);
            }
        }
    }

    // 4. Compute stream counts and ensure process totals match real throughput
    let mut list: Vec<ProcessTraffic> = process_map.into_values().collect();
    for p in &mut list {
        p.active_streams_count = p.streams.len();

        let sum_down: u64 = p.streams.iter().map(|s| s.down_speed_bps).sum();
        let sum_up: u64 = p.streams.iter().map(|s| s.up_speed_bps).sum();
        let sum_tot_down: u64 = p.streams.iter().map(|s| s.total_down_bytes).sum();
        let sum_tot_up: u64 = p.streams.iter().map(|s| s.total_up_bytes).sum();

        if p.down_speed_bps < sum_down {
            p.down_speed_bps = sum_down;
        }
        if p.up_speed_bps < sum_up {
            p.up_speed_bps = sum_up;
        }
        if p.total_down_bytes < sum_tot_down {
            p.total_down_bytes = sum_tot_down;
        }
        if p.total_up_bytes < sum_tot_up {
            p.total_up_bytes = sum_tot_up;
        }
    }

    // 5. Sort by active speed descending, then total bytes transferred, then streams count
    list.sort_by(|a, b| {
        let a_speed = a.down_speed_bps + a.up_speed_bps;
        let b_speed = b.down_speed_bps + b.up_speed_bps;
        if a_speed != b_speed {
            b_speed.cmp(&a_speed)
        } else {
            let a_tot = a.total_down_bytes + a.total_up_bytes;
            let b_tot = b.total_down_bytes + b.total_up_bytes;
            if a_tot != b_tot {
                b_tot.cmp(&a_tot)
            } else {
                b.active_streams_count.cmp(&a.active_streams_count)
            }
        }
    });

    list
}

#[cfg(windows)]
fn enforce_single_instance() -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE};

    unsafe {
        let mutex_name: Vec<u16> = "Global\\Flux_SingleInstance_Mutex\0".encode_utf16().collect();
        let _handle = CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let window_title: Vec<u16> = "Flux - Bandwidth Controller & Traffic Shaper\0".encode_utf16().collect();
            let hwnd = FindWindowW(std::ptr::null(), window_title.as_ptr());
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
            return false;
        }
    }
    true
}

fn main() {
    // Detect --minimized flag for silent autostart (hidden to system tray)
    let start_minimized = std::env::args().any(|a| a == "--minimized");

    #[cfg(windows)]
    {
        if !enforce_single_instance() {
            std::process::exit(0);
        }
    }

    let temp_log_path = std::env::temp_dir().join("flux.log");
    let _ = std::fs::write(&temp_log_path, format!("[STARTUP] Flux starting at {:?}\n", std::time::SystemTime::now()));

    // Set panic hook to log any panics to file
    let log_path_clone = temp_log_path.clone();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[PANIC] {}\n", info);
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&log_path_clone).unwrap();
        use std::io::Write;
        let _ = writeln!(file, "{}", msg);
    }));

    let is_admin = is_running_as_admin();
    let _ = std::fs::OpenOptions::new().append(true).open(&temp_log_path).map(|mut f| {
        use std::io::Write;
        let _ = writeln!(f, "[STARTUP] is_admin = {}", is_admin);
    });

    // Isolate WebView2 profile path by user integrity level to prevent 0x800700AA
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let profile_dir = if is_admin {
            std::path::PathBuf::from(local).join("com.tubbasoft.flux").join("webview_admin")
        } else {
            std::path::PathBuf::from(local).join("com.tubbasoft.flux").join("webview_user")
        };
        let _ = std::fs::create_dir_all(&profile_dir);
        let wv2_lock = profile_dir.join("EBWebView").join("lockfile");
        let _ = std::fs::remove_file(wv2_lock);
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &profile_dir);
        let _ = std::fs::OpenOptions::new().append(true).open(&temp_log_path).map(|mut f| {
            use std::io::Write;
            let _ = writeln!(f, "[STARTUP] WEBVIEW2_USER_DATA_FOLDER = {:?}", profile_dir);
        });
    }

    // Set WebView2 memory optimization arguments (cap V8 heap to 128MB, disable heavy unused background features)
    let current_wv2_args = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    let memory_args = "--disable-features=Translate,OptimizationHints,MediaRouter --disable-background-networking --disable-component-update --js-flags=\"--max-old-space-size=128\"";
    let combined_wv2_args = if current_wv2_args.is_empty() {
        memory_args.to_string()
    } else {
        format!("{} {}", current_wv2_args, memory_args)
    };
    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", combined_wv2_args);

    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let ip_resolver = Arc::new(IpHelperResolver::new());
    let shaper = Arc::new(TokenBucketShaper::new());

    let db = match Database::init() {
        Ok(d) => Arc::new(d),
        Err(e) => {
            eprintln!("Database initialization warning: {}", e);
            panic!("Database initialization failed: {}", e);
        }
    };

    // Load existing persistent rules into Token Bucket Shaper and fast In-Memory Caches
    let app_rules_cache = Arc::new(DashMap::new());
    for rule in db.get_all_app_rules() {
        shaper.update_process_rule(
            &rule.path,
            rule.is_blocked,
            rule.down_limit_kbps,
            rule.up_limit_kbps,
        );
        app_rules_cache.insert(rule.path.clone(), rule);
    }

    let stream_rules_cache = Arc::new(DashMap::new());
    for rule in db.get_all_stream_rules() {
        shaper.update_stream_rule(
            &rule.stream_id,
            rule.is_blocked,
            rule.down_limit_kbps,
            rule.up_limit_kbps,
        );
        stream_rules_cache.insert(rule.stream_id.clone(), rule);
    }

    let show_widget_init = db.get_setting("taskbar_widget", "true") == "true";
    let show_taskbar_widget = Arc::new(AtomicBool::new(show_widget_init));

    let detector = Arc::new(AppDetector::new(db.clone()));
    let packet_engine = Arc::new(PacketEngine::new(
        ip_resolver.clone(),
        shaper.clone(),
        db.clone(),
        detector.clone(),
    ));

    let is_admin = is_running_as_admin();
    let mut driver_loaded = false;
    let mut err_msg = None;
    let mut mode = "passive_monitor".to_string();

    if is_admin {
        match packet_engine.start() {
            Ok(_) => {
                info!("Kernel driver WinDivert started successfully!");
                driver_loaded = true;
                mode = "kernel_shaper".to_string();
            }
            Err(e) => {
                warn!("Could not start WinDivert driver: {}. Falling back to passive monitor.", e);
                err_msg = Some(e);
            }
        }
    } else {
        err_msg = Some("Administrator privileges required for full kernel diversion. Running in Active IP Helper Monitor Mode.".to_string());
    }

    let driver_status = Arc::new(RwLock::new(DriverStatus {
        is_admin,
        is_driver_loaded: driver_loaded,
        mode,
        packet_count: 0,
        diverted_bytes: 0,
        error_message: err_msg,
    }));

    let app_state = AppState {
        ip_resolver: ip_resolver.clone(),
        shaper: shaper.clone(),
        db: db.clone(),
        detector: detector.clone(),
        packet_engine: packet_engine.clone(),
        driver_status: driver_status.clone(),
        app_rules_cache: app_rules_cache.clone(),
        stream_rules_cache: stream_rules_cache.clone(),
        show_taskbar_widget: show_taskbar_widget.clone(),
    };

    // Start background IP Helper polling loop (every 500ms)
    {
        let ip_resolver = ip_resolver.clone();
        std::thread::spawn(move || loop {
            ip_resolver.refresh();
            std::thread::sleep(Duration::from_millis(500));
        });
    }

    tauri::Builder::default()
        .manage(app_state)
        .setup(move |app| {
            let handle = app.handle().clone();
            detector.set_app_handle(handle.clone());

            // 1. Initialize Taskbar Widget Window positioning & visibility
            let db_ref = app.state::<AppState>().db.clone();
            let show_widget = db_ref.get_setting("taskbar_widget", "true") == "true";
            if let Some(widget) = app.get_webview_window("widget") {
                position_widget_window(&widget, &db_ref);
                if show_widget {
                    let _ = widget.show();
                    let _ = widget.set_always_on_top(true);
                }
            }

            // Delayed retry for autostart / boot: ensure DWM and Explorer taskbar are fully loaded
            {
                let handle_for_widget_init = handle.clone();
                let db_for_widget_init = db_ref.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(1500));
                    let show_w = db_for_widget_init.get_setting("taskbar_widget", "true") == "true";
                    if show_w {
                        if let Some(w) = handle_for_widget_init.get_webview_window("widget") {
                            position_widget_window(&w, &db_for_widget_init);
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_always_on_top(true);
                        }
                    }
                });
            }

            // If launched with --minimized (autostart), keep main window hidden to system tray; otherwise show it
            if start_minimized {
                info!("[Autostart] Starting minimized to system tray...");
                if let Some(main_win) = app.get_webview_window("main") {
                    let _ = main_win.hide();
                }
            } else {
                if let Some(main_win) = app.get_webview_window("main") {
                    let _ = main_win.show();
                    let _ = main_win.set_focus();
                }
            }

            // 2. Setup System Tray
            let quit_item = MenuItem::with_id(app, "quit", "خروج نهائي (Exit)", true, None::<&str>)?;
            let widget_item = MenuItem::with_id(app, "toggle_widget", "ودجت شريط المهام (Taskbar Widget)", true, None::<&str>)?;
            let reset_widget_item = MenuItem::with_id(app, "reset_widget", "إعادة ضبط موضع الودجت (Reset Widget)", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "فتح Flux", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &widget_item, &reset_widget_item, &quit_item])?;

            let tray = TrayIconBuilder::with_id("main-tray")
                .menu(&tray_menu)
                .tooltip("Flux - Network Shaper")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                        }
                        "toggle_widget" => {
                            if let Some(w) = app.get_webview_window("widget") {
                                if let Ok(vis) = w.is_visible() {
                                    if vis {
                                        let _ = w.hide();
                                    } else {
                                        let db_inst = app.state::<AppState>().db.clone();
                                        position_widget_window(&w, &db_inst);
                                        let _ = w.show();
                                        let _ = w.unminimize();
                                        let _ = w.set_always_on_top(true);
                                    }
                                }
                            }
                        }
                        "reset_widget" => {
                            let db_inst = app.state::<AppState>().db.clone();
                            let _ = db_inst.set_setting("widget_x", "");
                            let _ = db_inst.set_setting("widget_y", "");
                            let _ = db_inst.set_setting("taskbar_widget", "true");
                            if let Some(w) = app.get_webview_window("widget") {
                                position_widget_window(&w, &db_inst);
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_always_on_top(true);
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if let Ok(vis) = w.is_visible() {
                                if vis {
                                    let _ = w.hide();
                                } else {
                                    let _ = w.show();
                                    let _ = w.unminimize();
                                    let _ = w.set_focus();
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            // 3. Real-time telemetry streaming task (500ms ticker, visibility-aware)
            let state_handle = app.state::<AppState>();
            let pe = state_handle.packet_engine.clone();
            let state_for_telemetry = state_handle.inner().clone();

            std::thread::spawn(move || {
                let mut last_instant = std::time::Instant::now();
                let mut emit_counter: u64 = 0;
                let mut last_recorded_down: u64 = 0;
                let mut last_recorded_up: u64 = 0;
                let mut last_tree_emit = std::time::Instant::now();
                loop {
                    std::thread::sleep(Duration::from_millis(500));
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(last_instant).as_secs_f64();
                    last_instant = now;

                    pe.compute_speed_window(elapsed);
                    let (g_down_speed, g_up_speed, g_tot_down, g_tot_up) = pe.get_global_speeds();

                    let delta_d = g_tot_down.saturating_sub(last_recorded_down);
                    let delta_u = g_tot_up.saturating_sub(last_recorded_up);
                    if delta_d > 0 || delta_u > 0 {
                        let _ = state_for_telemetry.db.add_daily_traffic(delta_d, delta_u);
                        last_recorded_down = g_tot_down;
                        last_recorded_up = g_tot_up;
                    }
                    let (today_d, today_u) = state_for_telemetry.db.get_today_traffic();

                    let global_telemetry = GlobalTelemetry {
                        down_speed_bps: g_down_speed,
                        up_speed_bps: g_up_speed,
                        total_down_bytes: g_tot_down,
                        total_up_bytes: g_tot_up,
                        today_down_bytes: today_d,
                        today_up_bytes: today_u,
                    };

                    emit_counter += 1;
                    if emit_counter % 2 == 0 {
                        let d_lbl = format_speed_label(g_down_speed);
                        let u_lbl = format_speed_label(g_up_speed);
                        let _ = tray.set_tooltip(Some(format!("Flux | ↓ {} | ↑ {}", d_lbl, u_lbl)));
                    }

                    // Keep widget visible and unminimized if user pressed Win+D, without disruptive restacking
                    if emit_counter % 6 == 0 {
                        if state_for_telemetry.show_taskbar_widget.load(Ordering::Relaxed) {
                            if let Some(widget) = handle.get_webview_window("widget") {
                                if let Ok(true) = widget.is_minimized() {
                                    let _ = widget.unminimize();
                                    let _ = widget.show();
                                    let _ = widget.set_always_on_top(true);
                                }
                            }
                        }
                    }

                    // Always emit lightweight global-telemetry for widget & tray
                    let _ = handle.emit("global-telemetry", &global_telemetry);

                    // Check if main window is visible before building/emitting heavy telemetry tree!
                    let is_main_visible = handle
                        .get_webview_window("main")
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false);

                    // Only build and emit heavy process/stream tree if main window is visible AND at 1-second intervals!
                    if is_main_visible && now.duration_since(last_tree_emit).as_millis() >= 1000 {
                        last_tree_emit = now;
                        let tree = build_telemetry_tree(&state_for_telemetry);
                        if emit_counter % 12 == 0 {
                            let total_streams: usize = tree.iter().map(|p| p.streams.len()).sum();
                            info!("[Telemetry] Emitting {} processes, {} streams to frontend (Live Down: {} bps, Up: {} bps)", tree.len(), total_streams, g_down_speed, g_up_speed);
                        }
                        let _ = handle.emit("telemetry-update", &tree);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let state = window.state::<AppState>();
                    let run_in_bg = state.db.get_setting("run_in_background", "true") == "true";
                    if run_in_bg {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_driver_status,
            get_telemetry_snapshot,
            get_global_telemetry,
            set_process_rule,
            set_stream_rule,
            open_file_location,
            terminate_process,
            close_stream_socket,
            request_admin_elevation,
            open_external_url,
            enforce_remote_lockdown,
            release_remote_lockdown,
            force_exit_app,
            get_autostart,
            set_autostart,
            get_run_in_background,
            set_run_in_background,
            get_taskbar_widget,
            toggle_taskbar_widget,
            save_widget_position,
            reset_widget_position,
            get_widget_config,
            save_widget_config,
            set_widget_preset,
            start_widget_drag,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application")
        .run(move |_app_handle, event| {
            let log_p = std::env::temp_dir().join("flux.log");
            let _ = std::fs::OpenOptions::new().append(true).open(&log_p).map(|mut f| {
                use std::io::Write;
                let _ = writeln!(f, "[EVENT] {:?}", event);
            });
        });
}
