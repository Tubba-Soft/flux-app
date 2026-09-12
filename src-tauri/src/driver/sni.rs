use std::net::IpAddr;
use std::sync::LazyLock;
use dashmap::DashMap;

pub static IP_DOMAIN_CACHE: LazyLock<DashMap<IpAddr, String>> = LazyLock::new(|| DashMap::new());

/// Parse TLS ClientHello SNI (Server Name Indication) from TCP payload
pub fn extract_sni_from_payload(payload: &[u8]) -> Option<String> {
    if payload.len() < 43 {
        return None;
    }

    // Byte 0: Content Type == 0x16 (Handshake)
    if payload[0] != 0x16 {
        return None;
    }

    // Byte 1-2: Version 3.x
    if payload[1] != 0x03 {
        return None;
    }

    // Byte 5: Handshake Type == 0x01 (ClientHello)
    if payload[5] != 0x01 {
        return None;
    }

    let mut offset = 5 + 4; // Handshake type (1) + length (3)
    if payload.len() < offset + 2 + 32 {
        return None;
    }

    // Skip client version (2) + random (32)
    offset += 2 + 32;

    // Session ID
    if offset >= payload.len() {
        return None;
    }
    let session_id_len = payload[offset] as usize;
    offset += 1 + session_id_len;

    // Cipher suites
    if offset + 2 > payload.len() {
        return None;
    }
    let cipher_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
    offset += 2 + cipher_len;

    // Compression methods
    if offset >= payload.len() {
        return None;
    }
    let comp_len = payload[offset] as usize;
    offset += 1 + comp_len;

    // Extensions length
    if offset + 2 > payload.len() {
        return None;
    }
    let extensions_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
    offset += 2;

    let extensions_end = (offset + extensions_len).min(payload.len());

    // Iterate extensions
    while offset + 4 <= extensions_end {
        let ext_type = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        let ext_len = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]) as usize;
        offset += 4;

        if offset + ext_len > extensions_end {
            break;
        }

        // ext_type == 0x0000 is server_name
        if ext_type == 0x0000 && ext_len >= 5 {
            let mut sni_offset = offset + 2; // skip server_name_list length (2 bytes)
            if sni_offset + 3 <= offset + ext_len {
                let name_type = payload[sni_offset];
                let name_len = u16::from_be_bytes([payload[sni_offset + 1], payload[sni_offset + 2]]) as usize;
                sni_offset += 3;

                if name_type == 0 && sni_offset + name_len <= offset + ext_len {
                    if let Ok(sni_str) = std::str::from_utf8(&payload[sni_offset..sni_offset + name_len]) {
                        return Some(sni_str.to_lowercase());
                    }
                }
            }
        }

        offset += ext_len;
    }

    None
}

/// Helper to associate an IP with a domain
pub fn record_domain_for_ip(ip: IpAddr, domain: String) {
    if !domain.is_empty() && !domain.contains("in-addr.arpa") {
        IP_DOMAIN_CACHE.insert(ip, domain);
    }
}

/// Lookup cached domain for IP
pub fn lookup_domain(ip: &IpAddr) -> Option<String> {
    IP_DOMAIN_CACHE.get(ip).map(|r| r.value().clone())
}
