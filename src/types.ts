export interface StreamTraffic {
  id: string;
  protocol: 'TCP' | 'UDP' | string;
  local_ip: string;
  local_port: number;
  remote_ip: string;
  remote_port: number;
  remote_domain?: string | null;
  down_speed_bps: number;
  up_speed_bps: number;
  total_down_bytes: number;
  total_up_bytes: number;
  is_blocked: boolean;
  down_limit_kbps?: number | null;
  up_limit_kbps?: number | null;
  state: string;
}

export interface ProcessTraffic {
  pid: number;
  name: string;
  path: string;
  icon_base64?: string | null;
  down_speed_bps: number;
  up_speed_bps: number;
  total_down_bytes: number;
  total_up_bytes: number;
  active_streams_count: number;
  is_blocked: boolean;
  down_limit_kbps?: number | null;
  up_limit_kbps?: number | null;
  priority: 'High' | 'Normal' | 'Low' | string;
  streams: StreamTraffic[];
}

export interface DriverStatus {
  is_admin: boolean;
  is_driver_loaded: boolean;
  mode: 'kernel_shaper' | 'passive_monitor' | string;
  packet_count: number;
  diverted_bytes: number;
  error_message?: string | null;
}

export interface AppRule {
  path: string;
  name: string;
  is_blocked: boolean;
  down_limit_kbps?: number | null;
  up_limit_kbps?: number | null;
  priority: string;
}

export interface StreamRule {
  stream_id: string;
  is_blocked: boolean;
  down_limit_kbps?: number | null;
  up_limit_kbps?: number | null;
}

export interface NewAppAlert {
  pid: number;
  name: string;
  path: string;
  remote_ip: string;
  remote_port: number;
  protocol: string;
  timestamp: number;
}

export type SortField = 'down_speed' | 'up_speed' | 'total_bytes' | 'name' | 'is_blocked' | 'is_throttled';
export type SortDirection = 'asc' | 'desc';
export type Language = 'en' | 'ar';

export interface GlobalTelemetry {
  down_speed_bps: number;
  up_speed_bps: number;
  total_down_bytes: number;
  total_up_bytes: number;
  today_down_bytes: number;
  today_up_bytes: number;
}

export interface WidgetConfig {
  locked: boolean;
  auto_transparency: boolean;
  transparency_delay_secs: number;
  transparency_opacity?: number;
  color_theme: string;
  preset: string;
}

