use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTraffic {
    pub pid: u32,
    pub name: String,
    pub path: String,
    pub icon_base64: Option<String>,
    pub down_speed_bps: u64,
    pub up_speed_bps: u64,
    pub total_down_bytes: u64,
    pub total_up_bytes: u64,
    pub active_streams_count: usize,
    pub is_blocked: bool,
    pub down_limit_kbps: Option<u64>,
    pub up_limit_kbps: Option<u64>,
    pub priority: String, // "High", "Normal", "Low"
    pub streams: Vec<StreamTraffic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamTraffic {
    pub id: String,
    pub protocol: String, // "TCP" or "UDP"
    pub local_ip: String,
    pub local_port: u16,
    pub remote_ip: String,
    pub remote_port: u16,
    pub remote_domain: Option<String>,
    pub down_speed_bps: u64,
    pub up_speed_bps: u64,
    pub total_down_bytes: u64,
    pub total_up_bytes: u64,
    pub is_blocked: bool,
    pub down_limit_kbps: Option<u64>,
    pub up_limit_kbps: Option<u64>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverStatus {
    pub is_admin: bool,
    pub is_driver_loaded: bool,
    pub mode: String, // "kernel_shaper" or "passive_monitor"
    pub packet_count: u64,
    pub diverted_bytes: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRule {
    pub path: String,
    pub name: String,
    pub is_blocked: bool,
    pub down_limit_kbps: Option<u64>,
    pub up_limit_kbps: Option<u64>,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRule {
    pub stream_id: String,
    pub is_blocked: bool,
    pub down_limit_kbps: Option<u64>,
    pub up_limit_kbps: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAppAlert {
    pub pid: u32,
    pub name: String,
    pub path: String,
    pub remote_ip: String,
    pub remote_port: u16,
    pub protocol: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTelemetry {
    pub down_speed_bps: u64,
    pub up_speed_bps: u64,
    pub total_down_bytes: u64,
    pub total_up_bytes: u64,
    pub today_down_bytes: u64,
    pub today_up_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub locked: bool,
    pub auto_transparency: bool,
    pub transparency_delay_secs: u32,
    pub transparency_opacity: u32,
    pub color_theme: String,
    pub preset: String,
}
