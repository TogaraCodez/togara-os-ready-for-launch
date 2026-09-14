//! PIC (8259) Interrupt Controller
//! 
//! Priority 8: Interrupt Handling
//! Classification: RUNTIME
//! 
//! Configures the Programmable Interrupt Controller for hardware interrupts.

use x86_64::instructions::port::Port;

/// PIC1 command port
const PIC1_COMMAND: u16 = 0x20;

/// PIC1 data port
const PIC1_DATA: u16 = 0x21;

/// PIC2 command port
const PIC2_COMMAND: u16 = 0xA0;

/// PIC2 data port
const PIC2_DATA: u16 = 0xA1;

/// End of interrupt command
const EOI: u8 = 0x20;

/// ICW1: Initialization command word 1
const ICW1_INIT: u8 = 0x11;

/// ICW4: Initialization command word 4
const ICW4_8086: u8 = 0x01;

/// PIC configuration
pub struct PicConfig {
    /// Offset for PIC1 (master)
    pub offset1: u8,
    /// Offset for PIC2 (slave)
    pub offset2: u8,
}

impl Default for PicConfig {
    fn default() -> Self {
        Self {
            offset1: 0x20, // IRQs 0-7 -> interrupts 0x20-0x27
            offset2: 0x28, // IRQs 8-15 -> interrupts 0x28-0x2F
        }
    }
}

/// PIC controller
pub struct Pic {
    config: PicConfig,
}

impl Pic {
    /// Create a new PIC instance
    pub const fn new(config: PicConfig) -> Self {
        Self { config }
    }

    /// Initialize the PIC
    /// 
    /// # Safety
    /// - Must be called before enabling interrupts
    /// - Configures hardware interrupt controller
    pub unsafe fn init(&mut self) {
        let mut port1 = Port::new(PIC1_COMMAND);
        let mut port2 = Port::new(PIC2_COMMAND);
        let mut data1 = Port::new(PIC1_DATA);
        let mut data2 = Port::new(PIC2_DATA);

        // Start initialization sequence
        port1.write(ICW1_INIT);
        port2.write(ICW1_INIT);

        // Set vector offsets
        data1.write(self.config.offset1);
        data2.write(self.config.offset2);

        // Tell Master PIC about Slave PIC
        data1.write(4);
        // Tell Slave PIC its cascade identity
        data2.write(2);

        // Set 8086 mode
        data1.write(ICW4_8086);
        data2.write(ICW4_8086);

        // Mask all interrupts initially
        data1.write(0xFF);
        data2.write(0xFF);
    }

    /// Send end-of-interrupt to PIC
    /// 
    /// # Safety
    /// - Must be called after handling hardware interrupt
    pub unsafe fn end_of_interrupt(&mut self, interrupt_id: u8) {
        if interrupt_id >= self.config.offset2 {
            Port::new(PIC2_COMMAND).write(EOI);
        }
        Port::new(PIC1_COMMAND).write(EOI);
    }

    /// Unmask an IRQ
    /// 
    /// # Safety
    /// - Enables hardware interrupt
    pub unsafe fn enable_irq(&mut self, irq: u8) {
        let port = if irq < 8 {
            Port::new(PIC1_DATA)
        } else {
            Port::new(PIC2_DATA)
        };

        let irq = irq % 8;
        let mask = port.read() & !(1 << irq);
        port.write(mask);
    }

    /// Mask an IRQ
    /// 
    /// # Safety
    /// - Disables hardware interrupt
    pub unsafe fn disable_irq(&mut self, irq: u8) {
        let port = if irq < 8 {
            Port::new(PIC1_DATA)
        } else {
            Port::new(PIC2_DATA)
        };

        let irq = irq % 8;
        let mask = port.read() | (1 << irq);
        port.write(mask);
    }
}

/// Timer IRQ (IRQ0)
pub const IRQ_TIMER: u8 = 0;

/// Keyboard IRQ (IRQ1)
pub const IRQ_KEYBOARD: u8 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pic_creation() {
        let config = PicConfig::default();
        let pic = Pic::new(config);
        assert_eq!(pic.config.offset1, 0x20);
        assert_eq!(pic.config.offset2, 0x28);
    }
}
