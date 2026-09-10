// rtl8139 nic driver polling version no interrupts

use crate::port::{inb, inw, outb, outw, outl};
use crate::tcp::pci::{self, PciDevice};
use spin::Mutex;

// register offsets from io_base
const REG_MAC0: u16 = 0x00;        // card mac 6 bytes
const REG_TSD0: u16 = 0x10;        // tx status descriptors 0 to 3
const REG_TSAD0: u16 = 0x20;       // tx address descriptors 0 to 3
const REG_RBSTART: u16 = 0x30;     // rx buffer address
const REG_CMD: u16 = 0x37;         // command register
const REG_CAPR: u16 = 0x38;        // read pointer
const REG_CBR: u16 = 0x3A;         // write pointer
const REG_IMR: u16 = 0x3C;         // interrupt mask
const REG_ISR: u16 = 0x3E;         // interrupt status
const REG_RCR: u16 = 0x44;         // rx config
const REG_CONFIG1: u16 = 0x52;     // card power config

const CMD_RESET: u8 = 0x10;
const CMD_RX_ENABLE: u8 = 0x08;
const CMD_TX_ENABLE: u8 = 0x04;
const ISR_ROK: u16 = 0x01;
const ISR_TOK: u16 = 0x04;

const RX_BUF_SIZE: usize = 8192 + 16 + 1500;

struct Rtl8139 {
    io_base: u16,
    rx_buffer: [u8; RX_BUF_SIZE],
    rx_offset: usize,
    tx_cur: usize,
    mac: [u8; 6],
}

static DRIVER: Mutex<Option<Rtl8139>> = Mutex::new(None);

// find card via pci and configure it returns false if absent
pub fn init() -> bool {
    let dev: PciDevice = match pci::find_device(pci::RTL8139_VENDOR, pci::RTL8139_DEVICE) {
        Some(d) => d,
        None => return false,
    };

    // enable dma
    pci::enable_bus_mastering(&dev);

    // bar0 to base io port mask flag bits
    let io_base = (dev.bar0 & 0xFFFC) as u16;

    let mut drv = Rtl8139 {
        io_base,
        rx_buffer: [0; RX_BUF_SIZE],
        rx_offset: 0,
        tx_cur: 0,
        mac: [0; 6],
    };

    unsafe {
        // power up the card
        outb(io_base + REG_CONFIG1, 0x00);

        // software reset wait for reset bit to clear
        outb(io_base + REG_CMD, CMD_RESET);
        let mut tries = 0;
        while inb(io_base + REG_CMD) & CMD_RESET != 0 {
            tries += 1;
            if tries > 100_000 { break; }
            core::hint::spin_loop();
        }

        // rx buffer address no paging yet so its virtual
        let rx_ptr = drv.rx_buffer.as_ptr() as u32;
        outl(io_base + REG_RBSTART, rx_ptr);

        // which events to catch
        outw(io_base + REG_IMR, ISR_ROK | ISR_TOK);

        // rx config accept all and wrap buffer
        outl(io_base + REG_RCR, 0x0F | (1 << 7));

        // enable rx and tx
        outb(io_base + REG_CMD, CMD_RX_ENABLE | CMD_TX_ENABLE);

        // read mac address
        for i in 0..6 {
            drv.mac[i] = inb(io_base + REG_MAC0 + i as u16);
        }
    }

    *DRIVER.lock() = Some(drv);
    true
}

// return card mac address
pub fn mac_address() -> Option<[u8; 6]> {
    DRIVER.lock().as_ref().map(|d| d.mac)
}

// send a frame by polling true on success
pub fn send(data: &[u8]) -> bool {
    let mut guard = DRIVER.lock();
    let drv = match guard.as_mut() {
        Some(d) => d,
        None => return false,
    };
    if data.len() > 1792 {
        return false;
    }
    let io = drv.io_base;
    let cur = drv.tx_cur;
    unsafe {
        let addr = data.as_ptr() as u32;
        outl(io + REG_TSAD0 + (cur * 4) as u16, addr);
        outl(io + REG_TSD0 + (cur * 4) as u16, data.len() as u32);
    }
    drv.tx_cur = (cur + 1) % 4;
    true
}

// fetch one frame by polling none if empty ring buffer format status length data crc
pub fn receive(out: &mut [u8]) -> Option<usize> {
    let mut guard = DRIVER.lock();
    let drv = guard.as_mut()?;
    let io = drv.io_base;
    unsafe {
        // clear isr flags or qemu wont deliver next packets
        let isr = inw(io + REG_ISR);
        if isr != 0 {
            outw(io + REG_ISR, isr);
        }

        // buffer empty bit in cmd
        if inb(io + REG_CMD) & 0x01 != 0 {
            return None;
        }

        let off = drv.rx_offset;

        // packet header little endian
        let status = drv.rx_buffer[off] as u16
            | ((drv.rx_buffer[off + 1] as u16) << 8);
        let length = drv.rx_buffer[off + 2] as u16
            | ((drv.rx_buffer[off + 3] as u16) << 8);

        // validity check rok bit plus sane length bounds
        let rx_ok = status & 0x01 != 0;
        if !rx_ok || length < 4 || length as usize > 2048 {
            // bad packet reset receive
            drv.rx_offset = 0;
            outw(io + REG_CAPR, (0u16).wrapping_sub(0x10));
            return None;
        }

        // copy data without header and crc
        let frame_len = (length as usize).saturating_sub(4); // drop crc
        let data_start = off + 4;                             // skip header
        let n = frame_len.min(out.len());
        for i in 0..n {
            // index wraps around the 8192 ring
            out[i] = drv.rx_buffer[(data_start + i) % 8192];
        }

        // advance offset to next packet align and wrap
        drv.rx_offset = ((off + length as usize + 4 + 3) & !3) % 8192;

        // capr must be offset minus 16 or receive stalls after first packet
        outw(io + REG_CAPR, (drv.rx_offset as u16).wrapping_sub(0x10));

        Some(n)
    }
}
