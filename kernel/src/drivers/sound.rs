use crate::port::{inb, outb};

const DSP_BASE: u16 = 0x220;

const DSP_RESET: u16 = DSP_BASE + 0x6;
const DSP_READ: u16 = DSP_BASE + 0xA;
const DSP_WRITE: u16 = DSP_BASE + 0xC;
const DSP_WRITE_STATUS: u16 = DSP_BASE + 0xC;
const DSP_READ_STATUS: u16 = DSP_BASE + 0xE;

const DMA_MASK: u16 = 0x0A;
const DMA_MODE: u16 = 0x0B;
const DMA_CLEAR_FF: u16 = 0x0C;
const DMA_CH1_ADDR: u16 = 0x02;
const DMA_CH1_COUNT: u16 = 0x03;
const DMA_CH1_PAGE: u16 = 0x83;

fn io_wait() {
    outb(0x80, 0);
}

fn spin_delay(loops: u32) {
    let mut count = loops;
    while count > 0 {
        core::hint::spin_loop();
        count -= 1;
    }
}

fn dsp_write(value: u8) {
    let mut tries: u32 = 10_000;
    while inb(DSP_WRITE_STATUS) & 0x80 != 0 {
        io_wait();
        tries -= 1;
        if tries == 0 {
            return;
        }
    }
    outb(DSP_WRITE, value);
}

fn dsp_read() -> u8 {
    let mut tries: u32 = 10_000;
    while inb(DSP_READ_STATUS) & 0x80 == 0 {
        io_wait();
        tries -= 1;
        if tries == 0 {
            return 0;
        }
    }
    inb(DSP_READ)
}

pub fn reset_dsp() -> bool {
    outb(DSP_RESET, 1);
    for _ in 0..30 {
        io_wait();
    }
    outb(DSP_RESET, 0);

    dsp_read() == 0xAA
}

pub fn set_sample_rate(rate: u16) {
    dsp_write(0x41);
    dsp_write((rate >> 8) as u8);
    dsp_write((rate & 0xFF) as u8);
}

pub fn program_dma8(addr: u32, length: u16) {
    outb(DMA_MASK, 0x04 | 1);
    outb(DMA_CLEAR_FF, 0);
    outb(DMA_MODE, 0x48 | 1);

    outb(DMA_CH1_ADDR, (addr & 0xFF) as u8);
    outb(DMA_CH1_ADDR, ((addr >> 8) & 0xFF) as u8);
    outb(DMA_CH1_PAGE, ((addr >> 16) & 0xFF) as u8);

    let count = length.wrapping_sub(1);
    outb(DMA_CH1_COUNT, (count & 0xFF) as u8);
    outb(DMA_CH1_COUNT, ((count >> 8) & 0xFF) as u8);

    outb(DMA_MASK, 1);
}

pub fn play_pcm8(addr: u32, length: u16, rate: u16) {
    set_sample_rate(rate);
    program_dma8(addr, length);

    let count = length.wrapping_sub(1);
    dsp_write(0x14);
    dsp_write((count & 0xFF) as u8);
    dsp_write(((count >> 8) & 0xFF) as u8);
}

pub fn stop() {
    dsp_write(0xD0);
    outb(DMA_MASK, 0x04 | 1);
}

// На будущее
pub fn irq_ack() {
    let _ = inb(DSP_READ_STATUS);
}

const BEEP_SAMPLE_RATE: u16 = 8000;
const BEEP_BUF_LEN: usize = 512;

static mut BEEP_BUF_PHYS: u64 = 0;
static mut BEEP_BUF_PTR: *mut u8 = core::ptr::null_mut();

fn ensure_beep_buffer() -> bool {
    unsafe {
        if !BEEP_BUF_PTR.is_null() {
            return true;
        }

        let Some(phys) = crate::mem::allocator::alloc_frames(1) else {
            return false;
        };

        const ISA_DMA_LIMIT: u64 = 0x0100_0000;
        if phys >= ISA_DMA_LIMIT {
            return false;
        }

        let Some(virt_ptr) = crate::mem::allocator::phys_to_virt(phys) else {
            return false;
        };

        BEEP_BUF_PHYS = phys;
        BEEP_BUF_PTR = virt_ptr;
        true
    }
}

pub fn init() -> bool {
    let ok = reset_dsp();
    if ok {
        dsp_write(0xD1);
    }
    let buf_ok = ensure_beep_buffer();
    ok && buf_ok
}

fn fill_square_wave(freq: u32) {
    let freq = if freq == 0 { 1 } else { freq };
    let half_period = ((BEEP_SAMPLE_RATE as u32) / (freq * 2)).max(1) as usize;

    unsafe {
        let buf = core::slice::from_raw_parts_mut(BEEP_BUF_PTR, BEEP_BUF_LEN);
        for i in 0..BEEP_BUF_LEN {
            buf[i] = if (i / half_period) % 2 == 0 { 255 } else { 0 };
        }
    }
}

fn program_dma8_autoinit(addr: u32, length: u16) {
    outb(DMA_MASK, 0x04 | 1);
    outb(DMA_CLEAR_FF, 0);

    outb(DMA_MODE, 0x58 | 1);

    outb(DMA_CH1_ADDR, (addr & 0xFF) as u8);
    outb(DMA_CH1_ADDR, ((addr >> 8) & 0xFF) as u8);
    outb(DMA_CH1_PAGE, ((addr >> 16) & 0xFF) as u8);

    let count = length.wrapping_sub(1);
    outb(DMA_CH1_COUNT, (count & 0xFF) as u8);
    outb(DMA_CH1_COUNT, ((count >> 8) & 0xFF) as u8);

    outb(DMA_MASK, 1);
}

pub fn play_freq(freq: u32) {
    if freq == 0 {
        stop();
        return;
    }

    stop();

    if !ensure_beep_buffer() {
        return;
    }

    fill_square_wave(freq);
    set_sample_rate(BEEP_SAMPLE_RATE);

    let addr = unsafe { BEEP_BUF_PHYS as u32 };
    program_dma8_autoinit(addr, BEEP_BUF_LEN as u16);

    let count = (BEEP_BUF_LEN as u16).wrapping_sub(1);
    dsp_write(0x48);
    dsp_write((count & 0xFF) as u8);
    dsp_write(((count >> 8) & 0xFF) as u8);
    dsp_write(0x1C);
}

pub fn delay(units: u32) {
    let mut count = units.wrapping_mul(200_000);
    while count > 0 {
        core::hint::spin_loop();
        count -= 1;
    }
}

pub fn beep(freq: u32, duration: u32) {
    play_freq(freq);
    delay(duration);
    stop();
}

pub const NOTE_C4: u32 = 262;
pub const NOTE_CS4: u32 = 277;
pub const NOTE_D4: u32 = 294;
pub const NOTE_DS4: u32 = 311;
pub const NOTE_E4: u32 = 330;
pub const NOTE_F4: u32 = 349;
pub const NOTE_FS4: u32 = 370;
pub const NOTE_G4: u32 = 392;
pub const NOTE_GS4: u32 = 415;
pub const NOTE_A4: u32 = 440;
pub const NOTE_AS4: u32 = 466;
pub const NOTE_B4: u32 = 494;
pub const NOTE_C5: u32 = 523;
pub const NOTE_D5: u32 = 587;
pub const NOTE_E5: u32 = 659;
pub const NOTE_F5: u32 = 698;
pub const NOTE_G5: u32 = 784;

const STREAM_HALF_LEN: usize = 2048;
const STREAM_BUF_LEN: usize = STREAM_HALF_LEN * 2;

static mut STREAM_BUF_PHYS: u64 = 0;
static mut STREAM_BUF_PTR: *mut u8 = core::ptr::null_mut();

static mut LAST_PLAYING_HALF: u8 = 0;

fn ensure_stream_buffer() -> bool {
    unsafe {
        if !STREAM_BUF_PTR.is_null() {
            return true;
        }
        let Some(phys) = crate::mem::allocator::alloc_frames(1) else {
            return false;
        };
        const ISA_DMA_LIMIT: u64 = 0x0100_0000;
        if phys >= ISA_DMA_LIMIT {
            return false;
        }
        let Some(virt_ptr) = crate::mem::allocator::phys_to_virt(phys) else {
            return false;
        };
        STREAM_BUF_PHYS = phys;
        STREAM_BUF_PTR = virt_ptr;
        true
    }
}

fn dma_current_count() -> u16 {
    outb(DMA_CLEAR_FF, 0);
    let lo = inb(DMA_CH1_COUNT) as u16;
    let hi = inb(DMA_CH1_COUNT) as u16;
    lo | (hi << 8)
}

fn current_playing_half() -> u8 {
    let count = dma_current_count() as u32;
    let max = (STREAM_BUF_LEN as u32).saturating_sub(1);
    let played = max.saturating_sub(count.min(max));
    if (played as usize) < STREAM_HALF_LEN { 0 } else { 1 }
}

pub struct SongPlayer {
    data: &'static [u8],
    pos: usize,
    silence_fills: u8,
}

impl SongPlayer {
    pub fn new(data: &'static [u8]) -> Self {
        Self { data, pos: 0, silence_fills: 0 }
    }

    fn fill_half(&mut self, half_index: u8) {
        unsafe {
            let offset = half_index as usize * STREAM_HALF_LEN;
            let dst = core::slice::from_raw_parts_mut(STREAM_BUF_PTR.add(offset), STREAM_HALF_LEN);

            let remaining = self.data.len().saturating_sub(self.pos);
            let take = remaining.min(STREAM_HALF_LEN);

            if take > 0 {
                dst[..take].copy_from_slice(&self.data[self.pos..self.pos + take]);
                self.pos += take;
            }
            if take < STREAM_HALF_LEN {
                for b in &mut dst[take..] {
                    *b = 128;
                }
            }

            self.silence_fills = if take == 0 {
                self.silence_fills.saturating_add(1)
            } else {
                0
            };
        }
    }

    pub fn finished(&self) -> bool {
        self.pos >= self.data.len()
    }

    pub fn drained(&self) -> bool {
        self.silence_fills >= 2
    }
}

pub fn stream_start(player: &mut SongPlayer, sample_rate: u16) -> bool {
    if !ensure_stream_buffer() {
        return false;
    }

    stop();
    set_sample_rate(sample_rate);

    player.fill_half(0);
    player.fill_half(1);
    unsafe { LAST_PLAYING_HALF = 0; }

    let addr = unsafe { STREAM_BUF_PHYS as u32 };
    program_dma8_autoinit(addr, STREAM_BUF_LEN as u16);

    let count = (STREAM_BUF_LEN as u16).wrapping_sub(1);
    dsp_write(0x48);
    dsp_write((count & 0xFF) as u8);
    dsp_write(((count >> 8) & 0xFF) as u8);
    dsp_write(0x1C);

    true
}

pub fn stream_tick(player: &mut SongPlayer) {
    let playing_half = current_playing_half();
    unsafe {
        if playing_half != LAST_PLAYING_HALF {
            let free_half = 1 - playing_half;
            player.fill_half(free_half);
            LAST_PLAYING_HALF = playing_half;
        }
    }
}
