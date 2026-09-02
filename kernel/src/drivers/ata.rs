use crate::port::{inb, inw, outb, outw};

const DATA: u16 = 0x1F0;
const SEC_COUNT: u16 = 0x1F2;
const LBA_LO: u16 = 0x1F3;
const LBA_MID: u16 = 0x1F4;
const LBA_HI: u16 = 0x1F5;
const DRIVE: u16 = 0x1F6;
const STATUS: u16 = 0x1F7;
const COMMAND: u16 = 0x1F7;

const CMD_READ: u8 = 0x20;
const CMD_WRITE: u8 = 0x30;
const CMD_FLUSH: u8 = 0xE7;

const ST_BSY: u8 = 0x80;
const ST_DRQ: u8 = 0x08;
const ST_ERR: u8 = 0x01;

pub const SECTOR_SIZE: usize = 512;

fn wait_not_busy() {
    // с таймаутом чтобы не зависнуть навечно если диска нет
    let mut tries = 0u32;
    loop {
        if inb(STATUS) & ST_BSY == 0 {
            break;
        }
        tries += 1;
        if tries > 100_000 {
            break;
        }
        core::hint::spin_loop();
    }
}

fn wait_drq() -> bool {
    let mut tries = 0u32;
    loop {
        let s = inb(STATUS);
        if s & ST_ERR != 0 {
            return false;
        }
        if s & ST_BSY == 0 && s & ST_DRQ != 0 {
            return true;
        }
        tries += 1;
        if tries > 100_000 {
            return false;
        }
        core::hint::spin_loop();
    }
}

// выставить lba и выбрать slave (бит 4 = 1)
fn select(lba: u32) {
    outb(DRIVE, 0xE0 | (((lba >> 24) & 0x0F) as u8));
    outb(SEC_COUNT, 1);
    outb(LBA_LO, (lba & 0xFF) as u8);
    outb(LBA_MID, ((lba >> 8) & 0xFF) as u8);
    outb(LBA_HI, ((lba >> 16) & 0xFF) as u8);
}

pub fn read_sector(lba: u32, buf: &mut [u8; SECTOR_SIZE]) -> bool {
    wait_not_busy();
    select(lba);
    outb(COMMAND, CMD_READ);

    if !wait_drq() {
        return false;
    }

    for i in 0..(SECTOR_SIZE / 2) {
        let word = inw(DATA);
        buf[i * 2] = (word & 0xFF) as u8;
        buf[i * 2 + 1] = (word >> 8) as u8;
    }
    true
}

pub fn write_sector(lba: u32, buf: &[u8; SECTOR_SIZE]) -> bool {
    wait_not_busy();
    select(lba);
    outb(COMMAND, CMD_WRITE);

    if !wait_drq() {
        return false;
    }

    for i in 0..(SECTOR_SIZE / 2) {
        let word = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
        outw(DATA, word);
    }

    outb(COMMAND, CMD_FLUSH);
    wait_not_busy();
    true
}
