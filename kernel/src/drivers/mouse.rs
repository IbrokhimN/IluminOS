// ps2 mouse driver mouse sits on second ps2 channel sends 3 byte packets
use crate::port::{inb, outb};
use spin::Mutex;

const DATA: u16 = 0x60;
const STATUS: u16 = 0x64;
const CMD: u16 = 0x64;

// mouse state position and buttons
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left: bool,
    pub right: bool,
}

static STATE: Mutex<MouseState> = Mutex::new(MouseState {
    x: 400,
    y: 300,
    left: false,
    right: false,
});

// wait until controller is ready for write
fn wait_write() {
    let mut t = 0;
    while inb(STATUS) & 2 != 0 {
        t += 1;
        if t > 100_000 {
            break;
        }
    }
}

// wait until data is ready to read
fn wait_read() {
    let mut t = 0;
    while inb(STATUS) & 1 == 0 {
        t += 1;
        if t > 100_000 {
            break;
        }
    }
}

// send command targeted at the mouse
fn write_mouse(val: u8) {
    wait_write();
    outb(CMD, 0xD4);
    wait_write();
    outb(DATA, val);
}

// read ack byte usually 0xfa
fn read_ack() -> u8 {
    wait_read();
    inb(DATA)
}

// mouse init
pub fn init() {
    // enable second ps2 channel
    wait_write();
    outb(CMD, 0xA8);

    // enable irq generation in controller config
    wait_write();
    outb(CMD, 0x20); // read config byte
    wait_read();
    let mut config = inb(DATA);
    config |= 2; // enable second channel interrupt
    config &= !0x20; // clear second channel disable bit
    wait_write();
    outb(CMD, 0x60); // write config byte
    wait_write();
    outb(DATA, config);

    // tell mouse to use default settings
    write_mouse(0xF6);
    read_ack();

    // enable packet streaming
    write_mouse(0xF4);
    read_ack();
}

// packet byte accumulator
static PACKET: Mutex<[u8; 3]> = Mutex::new([0; 3]);
static PACKET_IDX: Mutex<usize> = Mutex::new(0);

// poll mouse without blocking update state on full packet call often in gui loop
pub fn poll() {
    // check for data and that its mouse data
    let status = inb(STATUS);
    if status & 1 == 0 {
        return; // no data
    }
    if status & 0x20 == 0 {
        // keyboard data not mouse skip it
        return;
    }

    let byte = inb(DATA);
    let mut idx = PACKET_IDX.lock();
    let mut pkt = PACKET.lock();

    // first packet byte must have bit 3 set or we are out of sync
    if *idx == 0 && byte & 0x08 == 0 {
        return; // wait for a valid first byte
    }

    pkt[*idx] = byte;
    *idx += 1;

    if *idx >= 3 {
        *idx = 0;
        let flags = pkt[0];
        let dx = pkt[1];
        let dy = pkt[2];

        // offsets are signed use simple i8 version
        let mdx = dx as i8 as i32;
        let mdy = dy as i8 as i32;

        let mut s = STATE.lock();
        s.x += mdx;
        s.y -= mdy; // screen y grows down invert mouse y
        s.left = flags & 1 != 0;
        s.right = flags & 2 != 0;

        // clamp to screen bounds
        let (w, h) = crate::framebuffer::dimensions();
        if s.x < 0 {
            s.x = 0;
        }
        if s.y < 0 {
            s.y = 0;
        }
        if s.x > w as i32 - 1 {
            s.x = w as i32 - 1;
        }
        if s.y > h as i32 - 1 {
            s.y = h as i32 - 1;
        }
    }
}

// current state copy
pub fn get() -> (i32, i32, bool, bool) {
    let s = STATE.lock();
    (s.x, s.y, s.left, s.right)
}
