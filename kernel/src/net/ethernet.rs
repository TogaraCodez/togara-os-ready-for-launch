//! Ethernet Driver
//! 
//! Priority 4: Storage & Filesystem (Network)
//! Classification: RUNTIME
//! 
//! Ethernet frame handling.

/// Ethernet header size
pub const ETHERNET_HEADER_SIZE: usize = 14;

/// Ethernet frame
#[repr(C)]
pub struct EthernetFrame {
    pub dest_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ether_type: u16,
    pub payload: [u8; 1500],
}

impl EthernetFrame {
    /// Create new Ethernet frame
    pub fn new(dest_mac: [u8; 6], src_mac: [u8; 6], ether_type: u16) -> Self {
        Self {
            dest_mac,
            src_mac,
            ether_type: ether_type.to_be(),
            payload: [0; 1500],
        }
    }

    /// Get payload length
    pub fn payload_len(&self, total_len: usize) -> usize {
        total_len.saturating_sub(ETHERNET_HEADER_SIZE)
    }

    /// Get payload slice
    pub fn payload(&self, total_len: usize) -> &[u8] {
        let len = self.payload_len(total_len);
        &self.payload[..len]
    }
}

/// EtherType values
pub mod ether_types {
    pub const IPV4: u16 = 0x0800;
    pub const ARP: u16 = 0x0806;
    pub const IPV6: u16 = 0x86DD;
}

/// MAC address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    pub const fn broadcast() -> Self {
        Self([0xFF; 6])
    }

    pub const fn zero() -> Self {
        Self([0; 6])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethernet_frame() {
        let frame = EthernetFrame::new([0; 6], [0; 6], ether_types::IPV4);
        assert_eq!(frame.ether_type, 0x0800u16.to_be());
    }

    #[test]
    fn test_mac_address() {
        let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert_eq!(mac.0[0], 0x00);
        assert_eq!(mac.0[5], 0x55);
        
        let broadcast = MacAddress::broadcast();
        assert_eq!(broadcast.0, [0xFF; 6]);
    }
}
