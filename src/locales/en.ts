export const en = {
  app_name: "Flux",
  tagline: "Granular Bandwidth Controller & Traffic Shaper",
  
  // Remote Control & Kill Switch
  remote_locked_title: "Important Developer Notice",
  remote_locked_default_msg: "This version has been suspended or deprecated by the developer. Please download the latest version to continue.",
  remote_locked_action: "Download Update Now",
  remote_locked_exit: "Exit Application",
  tubba_soft_platform: "Tubba Soft Engineering Platform",
  
  // Theme Modes
  theme_light: "Light Mode",
  theme_dark: "Dark Mode",
  theme_system: "System Default",
  
  // Status Bar
  status_kernel_active: "Kernel Shaper Active",
  status_passive_monitor: "Passive Socket Monitor",
  status_admin_req: "Administrator Required for Kernel Shaping",
  status_elevate_btn: "Elevate to Admin",
  status_packets: "Packets Diverted",
  status_data_processed: "Shaped Volume",
  
  // Speed Header
  live_download: "Live Download",
  live_upload: "Live Upload",
  total_downloaded: "Total Down",
  total_uploaded: "Total Up",
  active_connections: "Active Sockets",
  tracked_apps: "Processes",
  
  // Filters & Controls
  search_placeholder: "Search process name, PID, IP, port, or domain...",
  sort_by: "Sort By:",
  sort_down_speed: "Download Speed",
  sort_up_speed: "Upload Speed",
  sort_total_bytes: "Total Data",
  sort_name: "Process Name (A-Z)",
  sort_blocked: "Blocked Apps First",
  sort_throttled: "Throttled Apps First",
  clear_filter: "Clear",
  
  // Table Columns
  col_application: "Application / Stream 5-Tuple",
  col_pid: "PID",
  col_down_speed: "Download Rate",
  col_up_speed: "Upload Rate",
  col_total_transfer: "Total Consumed",
  col_limits: "Bandwidth Shaper",
  col_block: "Block",
  col_actions: "Actions",

  // Stream Attributes
  stream_protocol: "Proto",
  stream_local: "Local Endpoint",
  stream_remote: "Remote Endpoint",
  stream_domain: "Domain / SNI",
  stream_close_socket: "Close Socket",
  stream_close_confirm: "Are you sure you want to terminate this socket connection?",
  
  // Rules & Limiter Modal
  rule_title_proc: "Traffic Shaper — Application Rule",
  rule_title_stream: "Traffic Shaper — Stream Rule",
  rule_target: "Target:",
  rule_down_limit: "Download Speed Cap",
  rule_up_limit: "Upload Speed Cap",
  rule_no_limit: "Unlimited",
  rule_block_down: "Drop Inbound Packets (Block Download)",
  rule_block_up: "Drop Outbound Packets (Block Upload)",
  rule_block_all: "Block All Network Traffic",
  rule_priority: "Traffic Priority",
  rule_priority_high: "High (VIP Prioritized)",
  rule_priority_normal: "Normal (Standard)",
  rule_priority_low: "Low (Background)",
  rule_save: "Apply Rules",
  rule_cancel: "Cancel",

  // Action Menu
  action_open_explorer: "Open File Location",
  action_inspect: "Inspect Properties",
  action_kill_proc: "Kill Process",
  action_kill_confirm: "Are you sure you want to forcibly terminate this process?",
  action_shape_traffic: "Configure Limits",
  
  // New App Alert
  alert_new_app_title: "New Network Activity Detected",
  alert_new_app_desc: "is attempting to access the network for the first time via",
  alert_allow: "Allow Traffic",
  alert_throttle: "Throttle...",
  alert_block: "Block Application",

  // Developer & AV Guidance
  av_notice_title: "Antivirus / Kernel Driver Notice",
  av_notice_text: "WinDivert 2.2-A is digitally signed by official developers. If your antivirus prompts a false positive, please whitelist the application directory.",
  
  // Language Switcher
  lang_toggle: "العربية",
  lang_current: "English",
};
