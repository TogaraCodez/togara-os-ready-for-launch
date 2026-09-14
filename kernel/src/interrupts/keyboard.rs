use spin::Mutex;

const QUEUE_SIZE: usize = 256;

struct ScancodeQueue {
    buffer: [u8; QUEUE_SIZE],
    read: usize,
    write: usize,
    count: usize,
}

impl ScancodeQueue {
    const fn new() -> Self {
        Self {
            buffer: [0; QUEUE_SIZE],
            read: 0,
            write: 0,
            count: 0,
        }
    }

    fn push(&mut self, scancode: u8) {
        if self.count >= QUEUE_SIZE {
            return;
        }

        self.buffer[self.write] = scancode;
        self.write = (self.write + 1) % QUEUE_SIZE;
        self.count += 1;
    }

    fn pop(&mut self) -> Option<u8> {
        if self.count == 0 {
            return None;
        }

        let scancode = self.buffer[self.read];
        self.read = (self.read + 1) % QUEUE_SIZE;
        self.count -= 1;

        Some(scancode)
    }
}

static SCANCODES: Mutex<ScancodeQueue> = Mutex::new(ScancodeQueue::new());

pub fn push_scancode(scancode: u8) {
    SCANCODES.lock().push(scancode);
}

pub fn pop_scancode() -> Option<u8> {
    SCANCODES.lock().pop()
}
