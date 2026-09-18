use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use parking_lot::RwLock;
use log::{info, warn, debug};

use crate::driver::windivert_ffi::*;
use crate::driver::sni::{extract_sni_from_payload, lookup_domain, record_domain_for_ip};
use crate::net::ip_helper::IpHelperResolver;
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
    handle: Arc<RwLock<Option<SafeWinDivertHandle>>>,
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
    db: Arc<Database>,
    detector: Arc<AppDetector>,
}

impl PacketEngine {
    pub fn new(
        ip_resolver: Arc<IpHelperResolver>,
        shaper: Arc<TokenBucketShaper>,
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
            handle: Arc::new(RwLock::new(None)),
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
            db,
            detector,
        }
    }

    /// Start the optimized WinDivert loop with CPU spike protection
    pub fn start(&self) -> Result<(), String> {
        let lib = match &self.divert_lib {
            Some(l) => l.clone(),
            None => return Err("WinDivert library not loaded".to_string()),
        };

        // 1. Targeted filter: Exclude loopback to avoid CPU spikes on internal IPC!
        let filter = "!loopback and (tcp or udp)";
        let handle = lib.open(filter, WINDIVERT_LAYER_NETWORK, 0, 0)?;

        // 2. Set queue length and time to prevent packet drops under heavy 100+ Mbps traffic
        lib.set_param(handle, WINDIVERT_PARAM_QUEUE_LENGTH, 16384);
        lib.set_param(handle, WINDIVERT_PARAM_QUEUE_TIME, 2000); // 2000 ms queue buffer
        lib.set_param(handle, WINDIVERT_PARAM_QUEUE_SIZE, 32 * 1024 * 1024); // 32 MB buffer

        *self.handle.write() = Some(handle);
        self.is_running.store(true, Ordering::SeqCst);

        let is_running = self.is_running.clone();
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

        let (delay_tx, delay_rx) = std::sync::mpsc::sync_channel::<DelayedPacket>(4096);
        let delay_lib = lib.clone();
        let is_running_delay = self.is_running.clone();

        // Dedicated background worker for delayed (throttled) packets so recv loop NEVER blocks!
        std::thread::spawn(move || {
            while is_running_delay.load(Ordering::Relaxed) {
                match delay_rx.recv_timeout(Duration::from_millis(10)) {
                    Ok(pkt) => {
                        let now = Instant::now();
                        if pkt.send_at > now {
                            std::thread::sleep(pkt.send_at - now);
                        }
                        let _ = delay_lib.send(handle, &pkt.data, &pkt.addr);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        // Dedicated packet processing loop
        std::thread::spawn(move || {
            info!("WinDivert packet loop started with targeted filter: {}", filter);

            let mut packet_buf = vec![0u8; 65535];
            let mut addr = WinDivertAddress::new();

            while is_running.load(Ordering::Relaxed) {
                match lib.recv(handle, &mut packet_buf, &mut addr) {
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

                            // Token Bucket evaluation
                            let decision = shaper.evaluate_packet(
                                exe_path.as_deref(),
                                &stream_id,
                                is_outbound,
                                packet_len,
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
                                        debug!("Delay queue full, dropping excess packet to preserve network stability");
                                    }
                                }
                                ShaperDecision::Pass => {
                                    if let Err(e) = lib.send(handle, packet_slice, &addr) {
                                        warn!("WinDivert send error: {}", e);
                                    }
                                }
                            }
                        } else {
                            if let Err(e) = lib.send(handle, packet_slice, &addr) {
                                warn!("WinDivert send (passthrough) error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        if is_running.load(Ordering::Relaxed) {
                            warn!("WinDivert recv error: {}", e);
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                }
            }

            lib.close(handle);
            info!("WinDivert packet loop stopped cleanly");
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.write().take() {
            if let Some(lib) = &self.divert_lib {
                lib.close(h);
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
