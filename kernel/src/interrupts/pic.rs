use pic8259::ChainedPics;
use spin::Mutex;

use super::{PIC_1_OFFSET, PIC_2_OFFSET};

pub const PIC_1: u8 = PIC_1_OFFSET;
pub const PIC_2: u8 = PIC_2_OFFSET;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1, PIC_2) });

pub fn init() {
    unsafe {
        PICS.lock().initialize();
    }
}
