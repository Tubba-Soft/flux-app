use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use parking_lot::RwLock;
use log::{debug, info, warn};

use crate::net::vpn_detect::is_vpn_process_path;

pub enum ShaperDecision {
    Pass,
    Drop,
    Delay(Duration),
}

#[derive(Debug, Clone)]
struct Bucket {
    rate_bytes_sec: f64,
    capacity: f64,
    tokens: f64,
    last_update: Instant,
}

impl Bucket {
    fn new(rate_kbps: u64) -> Self {
        let rate_bytes_sec = (rate_kbps * 1024) as f64;
        // Capacity allows a burst of up to 150ms of traffic or at least 64KB
        let capacity = (rate_bytes_sec * 0.15).max(65536.0);
        Self {
            rate_bytes_sec,
            capacity,
            tokens: capacity,
            last_update: Instant::now(),
        }
    }

    fn update_rate(&mut self, rate_kbps: u64) {
        let new_rate = (rate_kbps * 1024) as f64;
        self.capacity = (new_rate * 0.15).max(65536.0);
        self.rate_bytes_sec = new_rate;
    }

    fn consume(&mut self, packet_size: usize) -> ShaperDecision {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        // Refill tokens
        self.tokens = (self.tokens + elapsed * self.rate_bytes_sec).min(self.capacity);

        let size = packet_size as f64;
        if self.tokens >= size {
            self.tokens -= size;
            ShaperDecision::Pass
        } else {
            // Need more tokens: calculate delay
            let deficit = size - self.tokens;
            let wait_secs = deficit / self.rate_bytes_sec;

            // If required delay is within 500ms, queue the delay
            if wait_secs <= 0.5 {
                self.tokens -= size;
                // SAFETY: Clamp tokens to -capacity to prevent deep deficit spiral
                // that would cause all subsequent packets to be dropped until full recovery
                if self.tokens < -self.capacity {
                    self.tokens = -self.capacity;
                }
                ShaperDecision::Delay(Duration::from_secs_f64(wait_secs))
            } else {
                // Buffer overflow / too deep deficit -> drop packet to enforce strict cap
                ShaperDecision::Drop
            }
        }
    }
}

/// Essential ports that must NEVER be blocked on system processes to keep internet working.
/// DNS (53), DHCP (67, 68), NTP (123), LLMNR (5355), mDNS (5353), NetBIOS (137, 138, 139)
const CRITICAL_PORTS: &[u16] = &[53, 67, 68, 123, 5353, 5355, 137, 138, 139];

/// Check if a process path is a critical Windows system process (svchost, lsass, etc.)
#[inline]
fn is_critical_system_process(path: &str) -> bool {
    let lower = path.to_lowercase();
    let basename = lower.rsplit('\\').next().unwrap_or(&lower);
    matches!(
        basename,
        "svchost.exe" | "lsass.exe" | "csrss.exe" | "services.exe" | "wininit.exe" | "system"
    )
}

/// Check if either port in a connection is a critical infrastructure port
#[inline]
fn is_critical_port(local_port: u16, remote_port: u16) -> bool {
    CRITICAL_PORTS.contains(&local_port) || CRITICAL_PORTS.contains(&remote_port)
}

pub struct TokenBucketShaper {
    /// Inbound (download) process buckets: path -> RwLock<Bucket>
    process_down_buckets: DashMap<String, Arc<RwLock<Bucket>>>,
    /// Outbound (upload) process buckets: path -> RwLock<Bucket>
    process_up_buckets: DashMap<String, Arc<RwLock<Bucket>>>,
    /// Inbound stream buckets: stream_id -> RwLock<Bucket>
    stream_down_buckets: DashMap<String, Arc<RwLock<Bucket>>>,
    /// Outbound stream buckets: stream_id -> RwLock<Bucket>
    stream_up_buckets: DashMap<String, Arc<RwLock<Bucket>>>,

    /// Blocked paths
    blocked_paths: DashMap<String, bool>,
    /// Blocked streams
    blocked_streams: DashMap<String, bool>,
}

impl TokenBucketShaper {
    pub fn new() -> Self {
        Self {
            process_down_buckets: DashMap::new(),
            process_up_buckets: DashMap::new(),
            stream_down_buckets: DashMap::new(),
            stream_up_buckets: DashMap::new(),
            blocked_paths: DashMap::new(),
            blocked_streams: DashMap::new(),
        }
    }

    /// True when a rate limit exists. Shaping has no equivalent outside the packet path, so
    /// this always forces the engine to intercept.
    pub fn has_rate_limits(&self) -> bool {
        !self.process_down_buckets.is_empty()
            || !self.process_up_buckets.is_empty()
            || !self.stream_down_buckets.is_empty()
            || !self.stream_up_buckets.is_empty()
    }

    /// True when a single connection is blocked. A stream is a live 5-tuple with an
    /// ephemeral port, not an executable, so WFP's app-id filters cannot express it and it
    /// still has to be dropped on the packet path.
    pub fn has_blocked_streams(&self) -> bool {
        !self.blocked_streams.is_empty()
    }

    /// True when a whole application is blocked. Only forces interception where WFP
    /// blocking is unavailable; see `PacketEngine::desired_mode`.
    pub fn has_blocked_paths(&self) -> bool {
        !self.blocked_paths.is_empty()
    }

    /// Set process throttling and blocking rules
    pub fn update_process_rule(
        &self,
        path: &str,
        is_blocked: bool,
        down_limit_kbps: Option<u64>,
        up_limit_kbps: Option<u64>,
    ) {
        // A limit on a VPN client throttles the entire machine, because every other app's
        // traffic is carried inside that one tunnel. Store nothing and enforce nothing.
        if is_vpn_process_path(path) {
            warn!(
                "[Shaper] Refusing rule on VPN client {} - limiting the tunnel would throttle the whole machine",
                path
            );
            self.blocked_paths.remove(path);
            self.process_down_buckets.remove(path);
            self.process_up_buckets.remove(path);
            return;
        }

        if is_blocked {
            if is_critical_system_process(path) {
                info!("[Shaper] Blocking critical system process {} - DNS/DHCP/NTP ports will be auto-whitelisted", path);
            }
            self.blocked_paths.insert(path.to_string(), true);
        } else {
            self.blocked_paths.remove(path);
        }

        // Update download bucket
        if let Some(down_rate) = down_limit_kbps {
            if let Some(entry) = self.process_down_buckets.get(path) {
                entry.value().write().update_rate(down_rate);
            } else {
                self.process_down_buckets.insert(
                    path.to_string(),
                    Arc::new(RwLock::new(Bucket::new(down_rate))),
                );
            }
        } else {
            self.process_down_buckets.remove(path);
        }

        // Update upload bucket
        if let Some(up_rate) = up_limit_kbps {
            if let Some(entry) = self.process_up_buckets.get(path) {
                entry.value().write().update_rate(up_rate);
            } else {
                self.process_up_buckets.insert(
                    path.to_string(),
                    Arc::new(RwLock::new(Bucket::new(up_rate))),
                );
            }
        } else {
            self.process_up_buckets.remove(path);
        }
    }

    /// Set stream throttling and blocking rules
    pub fn update_stream_rule(
        &self,
        stream_id: &str,
        is_blocked: bool,
        down_limit_kbps: Option<u64>,
        up_limit_kbps: Option<u64>,
    ) {
        if is_blocked {
            self.blocked_streams.insert(stream_id.to_string(), true);
        } else {
            self.blocked_streams.remove(stream_id);
        }

        if let Some(down_rate) = down_limit_kbps {
            if let Some(entry) = self.stream_down_buckets.get(stream_id) {
                entry.value().write().update_rate(down_rate);
            } else {
                self.stream_down_buckets.insert(
                    stream_id.to_string(),
                    Arc::new(RwLock::new(Bucket::new(down_rate))),
                );
            }
        } else {
            self.stream_down_buckets.remove(stream_id);
        }

        if let Some(up_rate) = up_limit_kbps {
            if let Some(entry) = self.stream_up_buckets.get(stream_id) {
                entry.value().write().update_rate(up_rate);
            } else {
                self.stream_up_buckets.insert(
                    stream_id.to_string(),
                    Arc::new(RwLock::new(Bucket::new(up_rate))),
                );
            }
        } else {
            self.stream_up_buckets.remove(stream_id);
        }
    }

    /// Legacy evaluate_packet without port awareness (fallback, still works)
    pub fn evaluate_packet(
        &self,
        exe_path: Option<&str>,
        stream_id: &str,
        is_outbound: bool,
        packet_len: usize,
    ) -> ShaperDecision {
        self.evaluate_packet_with_ports(exe_path, stream_id, is_outbound, packet_len, 0, 0)
    }

    /// Evaluate packet with port-aware critical system protection.
    /// When a critical system process (svchost, lsass, etc.) is blocked, DNS/DHCP/NTP
    /// traffic is automatically passed through to prevent total internet outage.
    pub fn evaluate_packet_with_ports(
        &self,
        exe_path: Option<&str>,
        stream_id: &str,
        is_outbound: bool,
        packet_len: usize,
        local_port: u16,
        remote_port: u16,
    ) -> ShaperDecision {
        // 0. Tunnel traffic is never shaped. Delaying or dropping the encrypted outer flow
        //    breaks the tunnel for every application riding inside it.
        if let Some(path) = exe_path {
            if is_vpn_process_path(path) {
                return ShaperDecision::Pass;
            }
        }

        // 1. Check if entity is blocked
        if let Some(path) = exe_path {
            if self.blocked_paths.contains_key(path) {
                // CRITICAL SAFETY: If this is a system process (svchost, etc.),
                // automatically whitelist essential infrastructure ports (DNS, DHCP, NTP)
                // so blocking svchost stops Windows Update/telemetry but NOT internet!
                if is_critical_system_process(path) && is_critical_port(local_port, remote_port) {
                    // Allow this packet through - it's essential infrastructure
                    debug!("[Shaper] Allowing critical port {}:{} for blocked system process {}",
                        local_port, remote_port, path);
                } else {
                    return ShaperDecision::Drop;
                }
            }
        }
        if self.blocked_streams.contains_key(stream_id) {
            return ShaperDecision::Drop;
        }

        let mut max_delay = Duration::ZERO;

        // 2. Process-level rate limiting
        if let Some(path) = exe_path {
            let bucket_map = if is_outbound {
                &self.process_up_buckets
            } else {
                &self.process_down_buckets
            };

            if let Some(entry) = bucket_map.get(path) {
                let decision = entry.value().write().consume(packet_len);
                match decision {
                    ShaperDecision::Drop => return ShaperDecision::Drop,
                    ShaperDecision::Delay(d) => {
                        if d > max_delay {
                            max_delay = d;
                        }
                    }
                    ShaperDecision::Pass => {}
                }
            }
        }

        // 3. Stream-level rate limiting
        let stream_map = if is_outbound {
            &self.stream_up_buckets
        } else {
            &self.stream_down_buckets
        };

        if let Some(entry) = stream_map.get(stream_id) {
            let decision = entry.value().write().consume(packet_len);
            match decision {
                ShaperDecision::Drop => return ShaperDecision::Drop,
                ShaperDecision::Delay(d) => {
                    if d > max_delay {
                        max_delay = d;
                    }
                }
                ShaperDecision::Pass => {}
            }
        }

        if max_delay > Duration::ZERO {
            ShaperDecision::Delay(max_delay)
        } else {
            ShaperDecision::Pass
        }
    }
}
