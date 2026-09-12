use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use parking_lot::RwLock;
use log::{info, warn, error};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::NetworkManagement::IpHelper::*;
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};

/// Represents an active connection socket
#[derive(Debug, Clone)]
pub struct SocketEntry {
    pub protocol: String,
    pub local_ip: IpAddr,
    pub local_port: u16,
    pub remote_ip: IpAddr,
    pub remote_port: u16,
    pub pid: u32,
    pub state: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FiveTupleKey {
    pub is_tcp: bool,
    pub local_port: u16,
    pub remote_port: u16,
    pub local_ip: IpAddr,
    pub remote_ip: IpAddr,
}

pub struct IpHelperResolver {
    /// Exact TCP connection map: (local_port, remote_port) -> pid
    tcp_connections: DashMap<(u16, u16), u32>,
    /// TCP listen sockets: local_port -> pid
    tcp_listeners: DashMap<u16, u32>,
    /// UDP Ephemeral Cache with TTL: local_port -> (pid, last_seen_instant)
    udp_cache: DashMap<u16, (u32, Instant)>,
    /// Full socket snapshot for UI telemetry
    active_sockets: RwLock<Vec<SocketEntry>>,
    /// TTL for UDP sockets to prevent premature expiration on bursts
    udp_ttl: Duration,
}

impl IpHelperResolver {
    pub fn new() -> Self {
        Self {
            tcp_connections: DashMap::new(),
            tcp_listeners: DashMap::new(),
            udp_cache: DashMap::new(),
            active_sockets: RwLock::new(Vec::with_capacity(512)),
            udp_ttl: Duration::from_secs(8), // 8-second TTL for UDP attribution
        }
    }

    /// Refresh all TCP and UDP tables from Windows IP Helper API
    pub fn refresh(&self) {
        let mut sockets = Vec::with_capacity(512);

        // 1. Refresh TCP IPv4
        self.refresh_tcp4(&mut sockets);
        // 2. Refresh UDP IPv4
        self.refresh_udp4(&mut sockets);
        // 3. Refresh TCP IPv6
        self.refresh_tcp6(&mut sockets);
        // 4. Refresh UDP IPv6
        self.refresh_udp6(&mut sockets);

        // Prune expired UDP entries older than TTL
        let now = Instant::now();
        self.udp_cache.retain(|_, (_, timestamp)| {
            now.duration_since(*timestamp) <= self.udp_ttl
        });

        *self.active_sockets.write() = sockets;
    }

    fn refresh_tcp4(&self, list: &mut Vec<SocketEntry>) {
        let mut size: u32 = 0;
        unsafe {
            // Call once to get required buffer size
            GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                1, // sort
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
        }

        if size == 0 {
            return;
        }

        let mut buffer: Vec<u8> = vec![0u8; size as usize];
        let ret = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                1,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };

        if ret != NO_ERROR {
            return;
        }

        #[repr(C)]
        struct MibTcpRowOwnerPid {
            state: u32,
            local_addr: u32,
            local_port: u32,
            remote_addr: u32,
            remote_port: u32,
            owning_pid: u32,
        }

        let num_entries = unsafe { *(buffer.as_ptr() as *const u32) } as usize;
        let entries_ptr = unsafe { buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MibTcpRowOwnerPid };

        for i in 0..num_entries {
            let row = unsafe { &*entries_ptr.add(i) };
            let local_ip = IpAddr::V4(Ipv4Addr::from(u32::from_be(row.local_addr)));
            let remote_ip = IpAddr::V4(Ipv4Addr::from(u32::from_be(row.remote_addr)));
            let local_port = u16::from_be((row.local_port & 0xFFFF) as u16);
            let remote_port = u16::from_be((row.remote_port & 0xFFFF) as u16);
            let pid = row.owning_pid;

            let state_str = match row.state {
                1 => "CLOSED",
                2 => "LISTEN",
                3 => "SYN_SENT",
                4 => "SYN_RCVD",
                5 => "ESTABLISHED",
                6 => "FIN_WAIT1",
                7 => "FIN_WAIT2",
                8 => "CLOSE_WAIT",
                9 => "CLOSING",
                10 => "LAST_ACK",
                11 => "TIME_WAIT",
                12 => "DELETE_TCB",
                _ => "UNKNOWN",
            };

            if row.state == 2 {
                self.tcp_listeners.insert(local_port, pid);
            } else {
                self.tcp_connections.insert((local_port, remote_port), pid);
            }

            list.push(SocketEntry {
                protocol: "TCP".to_string(),
                local_ip,
                local_port,
                remote_ip,
                remote_port,
                pid,
                state: state_str.to_string(),
            });
        }
    }

    fn refresh_udp4(&self, list: &mut Vec<SocketEntry>) {
        let mut size: u32 = 0;
        unsafe {
            GetExtendedUdpTable(
                std::ptr::null_mut(),
                &mut size,
                1,
                AF_INET as u32,
                UDP_TABLE_OWNER_PID,
                0,
            );
        }

        if size == 0 {
            return;
        }

        let mut buffer: Vec<u8> = vec![0u8; size as usize];
        let ret = unsafe {
            GetExtendedUdpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                1,
                AF_INET as u32,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };

        if ret != NO_ERROR {
            return;
        }

        #[repr(C)]
        struct MibUdpRowOwnerPid {
            local_addr: u32,
            local_port: u32,
            owning_pid: u32,
        }

        let num_entries = unsafe { *(buffer.as_ptr() as *const u32) } as usize;
        let entries_ptr = unsafe { buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MibUdpRowOwnerPid };
        let now = Instant::now();

        for i in 0..num_entries {
            let row = unsafe { &*entries_ptr.add(i) };
            let local_ip = IpAddr::V4(Ipv4Addr::from(u32::from_be(row.local_addr)));
            let local_port = u16::from_be((row.local_port & 0xFFFF) as u16);
            let pid = row.owning_pid;

            // Cache in high-speed concurrent cache with timestamp
            self.udp_cache.insert(local_port, (pid, now));

            list.push(SocketEntry {
                protocol: "UDP".to_string(),
                local_ip,
                local_port,
                remote_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                remote_port: 0,
                pid,
                state: "ACTIVE".to_string(),
            });
        }
    }

    fn refresh_tcp6(&self, list: &mut Vec<SocketEntry>) {
        let mut size: u32 = 0;
        unsafe {
            GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                1,
                AF_INET6 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
        }

        if size == 0 {
            return;
        }

        let mut buffer: Vec<u8> = vec![0u8; size as usize];
        let ret = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                1,
                AF_INET6 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };

        if ret != NO_ERROR {
            return;
        }

        #[repr(C)]
        struct MibTcp6RowOwnerPid {
            local_addr: [u8; 16],
            dw_local_scope_id: u32,
            local_port: u32,
            remote_addr: [u8; 16],
            dw_remote_scope_id: u32,
            remote_port: u32,
            state: u32,
            owning_pid: u32,
        }

        let num_entries = unsafe { *(buffer.as_ptr() as *const u32) } as usize;
        let entries_ptr = unsafe { buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MibTcp6RowOwnerPid };

        for i in 0..num_entries {
            let row = unsafe { &*entries_ptr.add(i) };
            let local_ip = IpAddr::V6(Ipv6Addr::from(row.local_addr));
            let remote_ip = IpAddr::V6(Ipv6Addr::from(row.remote_addr));
            let local_port = u16::from_be((row.local_port & 0xFFFF) as u16);
            let remote_port = u16::from_be((row.remote_port & 0xFFFF) as u16);
            let pid = row.owning_pid;

            let state_str = match row.state {
                1 => "CLOSED",
                2 => "LISTEN",
                3 => "SYN_SENT",
                4 => "SYN_RCVD",
                5 => "ESTABLISHED",
                6 => "FIN_WAIT1",
                7 => "FIN_WAIT2",
                8 => "CLOSE_WAIT",
                9 => "CLOSING",
                10 => "LAST_ACK",
                11 => "TIME_WAIT",
                12 => "DELETE_TCB",
                _ => "UNKNOWN",
            };

            if row.state == 2 {
                self.tcp_listeners.insert(local_port, pid);
            } else {
                self.tcp_connections.insert((local_port, remote_port), pid);
            }

            list.push(SocketEntry {
                protocol: "TCP".to_string(),
                local_ip,
                local_port,
                remote_ip,
                remote_port,
                pid,
                state: state_str.to_string(),
            });
        }
    }

    fn refresh_udp6(&self, list: &mut Vec<SocketEntry>) {
        let mut size: u32 = 0;
        unsafe {
            GetExtendedUdpTable(
                std::ptr::null_mut(),
                &mut size,
                1,
                AF_INET6 as u32,
                UDP_TABLE_OWNER_PID,
                0,
            );
        }

        if size == 0 {
            return;
        }

        let mut buffer: Vec<u8> = vec![0u8; size as usize];
        let ret = unsafe {
            GetExtendedUdpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                1,
                AF_INET6 as u32,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };

        if ret != NO_ERROR {
            return;
        }

        #[repr(C)]
        struct MibUdp6RowOwnerPid {
            local_addr: [u8; 16],
            dw_local_scope_id: u32,
            local_port: u32,
            owning_pid: u32,
        }

        let num_entries = unsafe { *(buffer.as_ptr() as *const u32) } as usize;
        let entries_ptr = unsafe { buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MibUdp6RowOwnerPid };
        let now = Instant::now();

        for i in 0..num_entries {
            let row = unsafe { &*entries_ptr.add(i) };
            let local_ip = IpAddr::V6(Ipv6Addr::from(row.local_addr));
            let local_port = u16::from_be((row.local_port & 0xFFFF) as u16);
            let pid = row.owning_pid;

            self.udp_cache.insert(local_port, (pid, now));

            list.push(SocketEntry {
                protocol: "UDP".to_string(),
                local_ip,
                local_port,
                remote_ip: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                remote_port: 0,
                pid,
                state: "ACTIVE".to_string(),
            });
        }
    }

    /// Fast lookup: Resolve PID from packet 5-tuple
    pub fn resolve_pid(&self, is_tcp: bool, local_port: u16, remote_port: u16) -> Option<u32> {
        if is_tcp {
            // 1. Try exact local<->remote pair
            if let Some(entry) = self.tcp_connections.get(&(local_port, remote_port)) {
                return Some(*entry.value());
            }
            // 2. Reverse pair if direction inverted
            if let Some(entry) = self.tcp_connections.get(&(remote_port, local_port)) {
                return Some(*entry.value());
            }
            // 3. Fallback to local listener port
            if let Some(entry) = self.tcp_listeners.get(&local_port) {
                return Some(*entry.value());
            }
            if let Some(entry) = self.tcp_listeners.get(&remote_port) {
                return Some(*entry.value());
            }
            // 4. Quick reactive refresh for newly established TCP connections
            self.reactive_refresh_tcp(local_port, remote_port)
        } else {
            // UDP Ephemeral lookup from concurrent TTL cache
            if let Some(entry) = self.udp_cache.get(&local_port) {
                return Some(entry.value().0);
            }
            if let Some(entry) = self.udp_cache.get(&remote_port) {
                return Some(entry.value().0);
            }
            // If cache miss, do a quick reactive refresh for UDP
            if let Some(pid) = self.reactive_refresh_udp(local_port) {
                return Some(pid);
            }
            self.reactive_refresh_udp(remote_port)
        }
    }

    /// Reactive lookup to resolve newly established TCP connections before periodic refresh
    fn reactive_refresh_tcp(&self, local_port: u16, remote_port: u16) -> Option<u32> {
        let mut size: u32 = 0;
        unsafe {
            GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
        }
        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let ret = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };

        if ret != NO_ERROR {
            return None;
        }

        #[repr(C)]
        struct MibTcpRowOwnerPid {
            state: u32,
            local_addr: u32,
            local_port: u32,
            remote_addr: u32,
            remote_port: u32,
            owning_pid: u32,
        }

        let num_entries = unsafe { *(buffer.as_ptr() as *const u32) } as usize;
        let entries_ptr = unsafe { buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MibTcpRowOwnerPid };
        let mut found_pid = None;

        for i in 0..num_entries {
            let row = unsafe { &*entries_ptr.add(i) };
            let lp = u16::from_be((row.local_port & 0xFFFF) as u16);
            let rp = u16::from_be((row.remote_port & 0xFFFF) as u16);
            let pid = row.owning_pid;

            if row.state == 2 {
                self.tcp_listeners.insert(lp, pid);
            } else {
                self.tcp_connections.insert((lp, rp), pid);
            }

            if (lp == local_port && rp == remote_port)
                || (lp == remote_port && rp == local_port)
                || (row.state == 2 && (lp == local_port || lp == remote_port))
            {
                found_pid = Some(pid);
            }
        }

        found_pid
    }

    /// Reactive lookup to resolve sudden UDP packets from gaming / QUIC / Voice
    fn reactive_refresh_udp(&self, target_port: u16) -> Option<u32> {
        let mut size: u32 = 0;
        unsafe {
            GetExtendedUdpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET as u32,
                UDP_TABLE_OWNER_PID,
                0,
            );
        }
        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let ret = unsafe {
            GetExtendedUdpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                0,
                AF_INET as u32,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };

        if ret != NO_ERROR {
            return None;
        }

        #[repr(C)]
        struct MibUdpRowOwnerPid {
            local_addr: u32,
            local_port: u32,
            owning_pid: u32,
        }

        let num_entries = unsafe { *(buffer.as_ptr() as *const u32) } as usize;
        let entries_ptr = unsafe { buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MibUdpRowOwnerPid };
        let now = Instant::now();
        let mut found_pid = None;

        for i in 0..num_entries {
            let row = unsafe { &*entries_ptr.add(i) };
            let port = u16::from_be((row.local_port & 0xFFFF) as u16);
            self.udp_cache.insert(port, (row.owning_pid, now));
            if port == target_port {
                found_pid = Some(row.owning_pid);
            }
        }

        found_pid
    }

    /// Get current active socket snapshot for UI tree
    pub fn get_active_sockets(&self) -> Vec<SocketEntry> {
        self.active_sockets.read().clone()
    }

    /// Terminate a TCP connection by forcing state to MIB_TCP_STATE_DELETE_TCB
    pub fn close_tcp_socket(&self, local_ip: Ipv4Addr, local_port: u16, remote_ip: Ipv4Addr, remote_port: u16) -> Result<(), String> {
        #[repr(C)]
        struct MibTcpRow {
            dw_state: u32,
            dw_local_addr: u32,
            dw_local_port: u32,
            dw_remote_addr: u32,
            dw_remote_port: u32,
        }

        let mut row = MibTcpRow {
            dw_state: 12, // MIB_TCP_STATE_DELETE_TCB
            dw_local_addr: u32::from(local_ip).to_be(),
            dw_local_port: (local_port as u32).to_be() >> 16,
            dw_remote_addr: u32::from(remote_ip).to_be(),
            dw_remote_port: (remote_port as u32).to_be() >> 16,
        };

        let ret = unsafe {
            SetTcpEntry(&mut row as *mut _ as *mut _)
        };

        if ret == NO_ERROR {
            Ok(())
        } else {
            Err(format!("SetTcpEntry failed with error code: {}", ret))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_helper_real_sockets() {
        let resolver = IpHelperResolver::new();
        resolver.refresh();
        let sockets = resolver.get_active_sockets();
        println!("Real active sockets detected on system: {}", sockets.len());
        assert!(!sockets.is_empty(), "Expected active TCP/UDP sockets on Windows!");
        for s in sockets.iter().take(5) {
            println!("Socket: {} {}:{} -> {}:{} (PID: {}, State: {})",
                s.protocol, s.local_ip, s.local_port, s.remote_ip, s.remote_port, s.pid, s.state
            );
            assert!(s.local_port > 0);
        }
    }

    #[test]
    fn test_resolve_real_pids() {
        let resolver = IpHelperResolver::new();
        resolver.refresh();
        let sockets = resolver.get_active_sockets();
        let mut resolved_count = 0;
        let mut total_established = 0;
        for s in &sockets {
            if s.state == "ESTABLISHED" || s.state == "ACTIVE" {
                total_established += 1;
                let is_tcp = s.protocol == "TCP";
                let pid = resolver.resolve_pid(is_tcp, s.local_port, s.remote_port);
                println!("Testing resolve {}: local {} remote {} -> expected PID {}, resolved: {:?}", s.protocol, s.local_port, s.remote_port, s.pid, pid);
                if pid.is_some() && pid.unwrap() > 0 {
                    resolved_count += 1;
                }
            }
        }
        println!("Resolved {}/{} active connections!", resolved_count, total_established);
    }
}
