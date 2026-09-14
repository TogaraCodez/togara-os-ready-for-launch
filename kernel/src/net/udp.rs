//! UDP Transport Layer
//! 
//! Priority 4: Storage & Filesystem (Network)
//! Classification: RUNTIME
//! 
//! UDP datagram handling.

use super::ipv4::Ipv4Addr;

/// UDP header size
pub const UDP_HEADER_SIZE: usize = 8;

/// UDP header
#[repr(C)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dest_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    /// Create new UDP header
    pub fn new(src_port: u16, dest_port: u16, payload_len: u16) -> Self {
        Self {
            src_port: src_port.to_be(),
            dest_port: dest_port.to_be(),
            length: ((UDP_HEADER_SIZE as u16) + payload_len).to_be(),
            checksum: 0, // Optional in UDP over IPv4
        }
    }

    /// Get source port
    pub fn src_port(&self) -> u16 {
        u16::from_be(self.src_port)
    }

    /// Get destination port
    pub fn dest_port(&self) -> u16 {
        u16::from_be(self.dest_port)
    }

    /// Get length
    pub fn length(&self) -> u16 {
        u16::from_be(self.length)
    }
}

/// UDP endpoint
#[derive(Debug, Clone, Copy)]
pub struct UdpEndpoint {
    pub ip: Ipv4Addr,
    pub port: u16,
}

impl UdpEndpoint {
    pub const fn new(ip: Ipv4Addr, port: u16) -> Self {
        Self { ip, port }
    }

    pub const fn any(port: u16) -> Self {
        Self {
            ip: Ipv4Addr::any(),
            port,
        }
    }
}

/// Well-known ports
pub mod ports {
    pub const DNS: u16 = 53;
    pub const DHCP: u16 = 67;
    pub const HTTP: u16 = 80;
    pub const HTTPS: u16 = 443;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_header() {
        let header = UdpHeader::new(12345, 80, 100);
        assert_eq!(header.src_port(), 12345);
        assert_eq!(header.dest_port(), 80);
        assert_eq!(header.length(), 108);
    }

    #[test]
    fn test_udp_endpoint() {
        use super::super::ipv4::Ipv4Addr;
        let endpoint = UdpEndpoint::new(Ipv4Addr::localhost(), 8080);
        assert_eq!(endpoint.port, 8080);
    }
}
