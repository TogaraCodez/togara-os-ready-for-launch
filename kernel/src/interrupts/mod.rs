pub mod idt;
pub mod keyboard;
pub mod pic;
pub mod timer;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = 40;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

pub fn init() {
    timer::init();
    pic::init();
    idt::init();

    x86_64::instructions::interrupts::enable();
}
