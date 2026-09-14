use spin::Once;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use super::{InterruptIndex, keyboard, pic, timer};

static IDT: Once<InterruptDescriptorTable> = Once::new();

pub fn init() {
    IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_handler);

        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_handler);

        idt
    })
    .load();
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    timer::tick();

    unsafe {
        pic::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    let mut port = x86_64::instructions::port::Port::new(0x60);

    unsafe {
        let scancode: u8 = port.read();

        keyboard::push_scancode(scancode);
        crate::system::state::record_keyboard_event();
        pic::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
