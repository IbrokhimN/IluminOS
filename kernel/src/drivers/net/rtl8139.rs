// драйвер сетевой карты RTL8139 версия без прерываний опрос

use crate::port::{inb, inw, outb, outw, outl};
use crate::tcp::pci::{self, PciDevice};
use spin::Mutex;

// смещения регистров от io_base
const REG_MAC0: u16 = 0x00;        // MAC карты 6 байт
const REG_TSD0: u16 = 0x10;        // TX status дескрипторов 0..3
const REG_TSAD0: u16 = 0x20;       // TX адрес дескрипторов 0..3
const REG_RBSTART: u16 = 0x30;     // адрес буфера приёма
const REG_CMD: u16 = 0x37;         // командный регистр
const REG_CAPR: u16 = 0x38;        // докуда мы прочитали read ptr
const REG_CBR: u16 = 0x3A;         // докуда карта записала write ptr
const REG_IMR: u16 = 0x3C;         // маска прерываний
const REG_ISR: u16 = 0x3E;         // статус прерываний
const REG_RCR: u16 = 0x44;         // конфиг приёма
const REG_CONFIG1: u16 = 0x52;     // питание карты

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

// найти карту через PCI и настроить false если карты нет
pub fn init() -> bool {
    let dev: PciDevice = match pci::find_device(pci::RTL8139_VENDOR, pci::RTL8139_DEVICE) {
        Some(d) => d,
        None => return false,
    };

    // разрешить DMA
    pci::enable_bus_mastering(&dev);

    // BAR0 -> базовый IO порт маскируем биты флаги
    let io_base = (dev.bar0 & 0xFFFC) as u16;

    let mut drv = Rtl8139 {
        io_base,
        rx_buffer: [0; RX_BUF_SIZE],
        rx_offset: 0,
        tx_cur: 0,
        mac: [0; 6],
    };

    unsafe {
        // питание карты
        outb(io_base + REG_CONFIG1, 0x00);

        // программный сброс ждём пока бит reset сбросится
        outb(io_base + REG_CMD, CMD_RESET);
        let mut tries = 0;
        while inb(io_base + REG_CMD) & CMD_RESET != 0 {
            tries += 1;
            if tries > 100_000 { break; }
            core::hint::spin_loop();
        }

        // адрес буфера приёма пока без paging виртуальный
        let rx_ptr = drv.rx_buffer.as_ptr() as u32;
        outl(io_base + REG_RBSTART, rx_ptr);

        // какие события ловим
        outw(io_base + REG_IMR, ISR_ROK | ISR_TOK);

        // конфиг приёма принимать всё + wrap буфера
        outl(io_base + REG_RCR, 0x0F | (1 << 7));

        // включить приём и передачу
        outb(io_base + REG_CMD, CMD_RX_ENABLE | CMD_TX_ENABLE);

        // прочитать MAC
        for i in 0..6 {
            drv.mac[i] = inb(io_base + REG_MAC0 + i as u16);
        }
    }

    *DRIVER.lock() = Some(drv);
    true
}

// вернуть MAC карты
pub fn mac_address() -> Option<[u8; 6]> {
    DRIVER.lock().as_ref().map(|d| d.mac)
}

// отправить кадр опросом true при успехе
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

// забрать один кадр опросом None пусто
// карта пишет пакеты в кольцевой буфер формат
// [2 байта статус][2 байта длина][данные][CRC]
// CAPR докуда мы прочитали CBR докуда карта записала
pub fn receive(out: &mut [u8]) -> Option<usize> {
    let mut guard = DRIVER.lock();
    let drv = guard.as_mut()?;
    let io = drv.io_base;
    unsafe {
        // сбросить флаги ISR иначе на QEMU карта не отдаёт следующие пакеты
        let isr = inw(io + REG_ISR);
        if isr != 0 {
            outw(io + REG_ISR, isr);
        }

        // буфер пуст бит BUFE в CMD
        if inb(io + REG_CMD) & 0x01 != 0 {
            return None;
        }

        let off = drv.rx_offset;

        // заголовок пакета little endian
        let status = drv.rx_buffer[off] as u16
            | ((drv.rx_buffer[off + 1] as u16) << 8);
        let length = drv.rx_buffer[off + 2] as u16
            | ((drv.rx_buffer[off + 3] as u16) << 8);

        // валидность бит 0 ROK + здравые границы длины
        let rx_ok = status & 0x01 != 0;
        if !rx_ok || length < 4 || length as usize > 2048 {
            // битый пакет сбросить приём
            drv.rx_offset = 0;
            outw(io + REG_CAPR, (0u16).wrapping_sub(0x10));
            return None;
        }

        // скопировать данные без заголовка и CRC
        let frame_len = (length as usize).saturating_sub(4); // отбросить CRC
        let data_start = off + 4;                             // пропустить заголовок
        let n = frame_len.min(out.len());
        for i in 0..n {
            // wrap индекс заворачивается по кольцу 8192
            out[i] = drv.rx_buffer[(data_start + i) % 8192];
        }

        // сдвинуть offset на следующий пакет +4 заголовок выравнивание wrap
        drv.rx_offset = ((off + length as usize + 4 + 3) & !3) % 8192;

        // критично CAPR = offset - 16 иначе приём встаёт после первого пакета
        outw(io + REG_CAPR, (drv.rx_offset as u16).wrapping_sub(0x10));

        Some(n)
    }
}
