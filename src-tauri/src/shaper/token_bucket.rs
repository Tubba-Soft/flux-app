use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use parking_lot::RwLock;
use log::{debug, info};

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
                self.tokens -= size; // allow slight token deficit
                ShaperDecision::Delay(Duration::from_secs_f64(wait_secs))
            } else {
                // Buffer overflow / too deep deficit -> drop packet to enforce strict cap
                ShaperDecision::Drop
            }
        }
    }
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

    /// Set process throttling and blocking rules
    pub fn update_process_rule(
        &self,
        path: &str,
        is_blocked: bool,
        down_limit_kbps: Option<u64>,
        up_limit_kbps: Option<u64>,
    ) {
        if is_blocked {
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

    /// Evaluate packet through process-level and stream-level rules
    pub fn evaluate_packet(
        &self,
        exe_path: Option<&str>,
        stream_id: &str,
        is_outbound: bool,
        packet_len: usize,
    ) -> ShaperDecision {
        // 1. Check if entity is blocked
        if let Some(path) = exe_path {
            if self.blocked_paths.contains_key(path) {
                return ShaperDecision::Drop;
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
