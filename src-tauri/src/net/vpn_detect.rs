//! VPN / tunnel detection.
//!
//! Re-injecting a VPN client's own encrypted packets at the WinDivert NETWORK layer is
//! unsafe: `WinDivertSend` performs a fresh route lookup for outbound packets and does not
//! honour the `IP_UNICAST_IF` binding the VPN client set on its socket. With a full-tunnel
//! default route (`0.0.0.0/1` + `128.0.0.0/1`) the encrypted packet is routed back into the
//! tunnel it belongs to, and the tunnel dies.
//!
//! This module finds the VPN server endpoints so the WinDivert filter can exclude that
//! traffic outright. It deliberately does NOT exclude the tunnel adapter itself: the
//! plaintext side carries the real per-application PIDs and remote addresses that the whole
//! product is built on. Excluding only the encrypted side is also what removes the
//! double-counting that otherwise shows every byte twice.

use std::net::IpAddr;

use crate::net::ip_helper::IpHelperResolver;
use crate::process::win32_shell::{get_process_name_from_path, get_process_path};

/// Executables that terminate a tunnel. Their traffic is the encrypted outer flow and must
/// never be shaped, blocked or re-injected.
const VPN_PROCESS_NAMES: &[&str] = &[
    // Proton
    "protonvpn.exe",
    "protonvpnservice.exe",
    "protonvpn.wireguardservice.exe",
    "protonvpnservice.wireguard.exe",
    // WireGuard / OpenVPN
    "wireguard.exe",
    "wgnt.exe",
    "openvpn.exe",
    "openvpnserv.exe",
    "openvpnserv2.exe",
    "openvpn-gui.exe",
    // Consumer VPNs
    "nordvpn.exe",
    "nordvpn-service.exe",
    "nordvpnservice.exe",
    "expressvpn.exe",
    "expressvpnd.exe",
    "surfshark.exe",
    "surfsharkservice.exe",
    "mullvadvpn.exe",
    "mullvad-daemon.exe",
    "windscribe.exe",
    "windscribeservice.exe",
    // Mesh / corporate
    "tailscaled.exe",
    "tailscale-ipn.exe",
    "zerotier-one_x64.exe",
    "vpnagent.exe",
    "vpnui.exe",
    "forticlient.exe",
    "fortivpn.exe",
    "pangps.exe",
    "pandesktop.exe",
];

/// True when `path` is a VPN/tunnel client executable.
pub fn is_vpn_process_path(path: &str) -> bool {
    let name = get_process_name_from_path(path).to_lowercase();
    VPN_PROCESS_NAMES.contains(&name.as_str())
}

/// Remote endpoints of every socket currently owned by a VPN client, i.e. the VPN servers
/// this machine is tunnelling to. Sorted and de-duplicated so callers can compare two
/// snapshots for equality.
pub fn detect_vpn_endpoints(resolver: &IpHelperResolver) -> Vec<IpAddr> {
    let mut endpoints: Vec<IpAddr> = Vec::new();

    for socket in resolver.get_active_sockets() {
        if socket.pid <= 4 {
            continue;
        }
        // Unbound / listening sockets carry no server address.
        if socket.remote_port == 0 || socket.remote_ip.is_unspecified() {
            continue;
        }
        let Some(path) = get_process_path(socket.pid) else {
            continue;
        };
        if !is_vpn_process_path(&path) {
            continue;
        }
        if socket.remote_ip.is_loopback() {
            continue;
        }
        endpoints.push(socket.remote_ip);
    }

    endpoints.sort();
    endpoints.dedup();
    endpoints
}
