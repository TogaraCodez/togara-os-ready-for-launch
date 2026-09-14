//! IPv4 Network Layer
//! 
//! Priority 4: Storage & Filesystem (Network)
//! Classification: RUNTIME
//! 
//! IPv4 packet handling with checksum calculation.

use super::ethernet::MacAddress;

/// IPv4 header size
pub const IPV4_HEADER_SIZE: usize = 20;

/// IPv4 header
#[repr(C)]
pub struct Ipv4Header {
    pub version_ihl: u8,
    pub dscp_ecn: u8,
    pub total_length: u16,
    pub identification: u16,
    pub flags_fragment: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src_addr: u32,
    pub dest_addr: u32,
}

impl Ipv4Header {
    /// Create new IPv4 header
    pub fn new(src: Ipv4Addr, dest: Ipv4Addr, protocol: u8, payload_len: u16) -> Self {
        let mut header = Self {
            version_ihl: 0x45, // Version 4, IHL 5 (20 bytes)
            dscp_ecn: 0,
            total_length: ((IPV4_HEADER_SIZE as u16) + payload_len).to_be(),
            identification: 0,
            flags_fragment: 0,
            ttl: 64,
            protocol,
            checksum: 0, // Calculated below
            src_addr: src.to_bits(),
            dest_addr: dest.to_bits(),
        };
        
        // Calculate checksum
        header.checksum = header.calculate_checksum();
        
        header
    }

    /// Calculate header checksum
    pub fn calculate_checksum(&self) -> u16 {
        let mut sum: u32 = 0;
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                IPV4_HEADER_SIZE,
            )
        };
        
        for i in (0..IPV4_HEADER_SIZE).step_by(2) {
            let word = ((bytes[i] as u32) << 8) | (bytes[i + 1] as u32);
            sum += word;
        }
        
        // Fold 32-bit sum to 16 bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        !sum as u16
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {
        self.calculate_checksum() == 0
    }

    /// Get header length
    pub fn ihl(&self) -> usize {
        ((self.version_ihl & 0x0F) as usize) * 4
    }

    /// Get version
    pub fn version(&self) -> u8 {
        (self.version_ihl >> 4) & 0x0F
    }

    /// Get total length
    pub fn total_length(&self) -> u16 {
        u16::from_be(self.total_length)
    }

    /// Get source address
    pub fn src_addr(&self) -> Ipv4Addr {
        Ipv4Addr::from_bits(u32::from_be(self.src_addr))
    }

    /// Get destination address
    pub fn dest_addr(&self) -> Ipv4Addr {
        Ipv4Addr::from_bits(u32::from_be(self.dest_addr))
    }
}

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Addr(pub [u8; 4]);

impl Ipv4Addr {
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self([
            ((bits >> 24) & 0xFF) as u8,
            ((bits >> 16) & 0xFF) as u8,
            ((bits >> 8) & 0xFF) as u8,
            (bits & 0xFF) as u8,
        ])
    }

    pub const fn to_bits(self) -> u32 {
        ((self.0[0] as u32) << 24)
            | ((self.0[1] as u32) << 16)
            | ((self.0[2] as u32) << 8)
            | (self.0[3] as u32)
    }

    pub const fn localhost() -> Self {
        Self([127, 0, 0, 1])
    }

    pub const fn any() -> Self {
        Self([0, 0, 0, 0])
    }
}

/// IP protocols
pub mod protocols {
    pub const ICMP: u8 = 1;
    pub const TCP: u8 = 6;
    pub const UDP: u8 = 17;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_addr() {
        let addr = Ipv4Addr::new(192, 168, 1, 1);
        assert_eq!(addr.0, [192, 168, 1, 1]);
        assert_eq!(addr.to_bits(), 0xC0A80101);
        
        let from_bits = Ipv4Addr::from_bits(0x7F000001);
        assert_eq!(from_bits, Ipv4Addr::localhost());
    }

    #[test]
    fn test_ipv4_header() {
        let src = Ipv4Addr::localhost();
        let dest = Ipv4Addr::new(192, 168, 1, 1);
        let header = Ipv4Header::new(src, dest, protocols::UDP, 100);
        
        assert_eq!(header.version(), 4);
        assert_eq!(header.ihl(), 20);
        assert_eq!(header.total_length(), 120);
    }
}
