// сканер шины PCI находит устройства на материнке

use crate::port::{inl, outl};

// RTL8139 опознаётся по этой паре
pub const RTL8139_VENDOR: u16 = 0x10EC; // Realtek
pub const RTL8139_DEVICE: u16 = 0x8139;

// найденное PCI устройство
#[derive(Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub bar0: u32,      // базовый адрес
    pub irq_line: u8,   // номер IRQ
}

// собрать 32 битный адрес для порта 0xCF8
// формат задан железом enable bit + bus + device + function + смещение
fn make_address(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    (1 << 31)                        // enable bit
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC)   // выравнивание на 4 байта
}

// прочитать 32 битное поле из конфига устройства
pub fn config_read_u32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let addr = make_address(bus, device, function, offset);
    outl(0xCF8, addr);  // записали адрес
    inl(0xCFC)          // прочитали данные
}

// прочитать 16 битное поле половину u32
pub fn config_read_u16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let dword = config_read_u32(bus, device, function, offset);
    // offset & 2 выбирает какую половину взять
    ((dword >> ((offset as u32 & 2) * 8)) & 0xFFFF) as u16
}

// записать 32 битное поле
pub fn config_write_u32(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let addr = make_address(bus, device, function, offset);
    outl(0xCF8, addr);
    outl(0xCFC, value);
}

// пройти всю шину и найти устройство по vendor+device
pub fn find_device(vendor: u16, device_id_wanted: u16) -> Option<PciDevice> {
    // перебор всех адресов 256 шин * 32 устройства * 8 функций
    for bus in 0..256u16 {
        for device in 0..32u8 {
            for function in 0..8u8 {
                let bus = bus as u8;
                // читаем vendor_id
                let vid = config_read_u16(bus, device, function, 0x00);
                // 0xFFFF пустой слот
                if vid == 0xFFFF {
                    continue;
                }
                let did = config_read_u16(bus, device, function, 0x02);

                // то что ищем
                if vid == vendor && did == device_id_wanted {
                    // BAR0 базовый адрес карты
                    let bar0 = config_read_u32(bus, device, function, 0x10);
                    // IRQ line младший байт
                    let irq = (config_read_u32(bus, device, function, 0x3C) & 0xFF) as u8;

                    return Some(PciDevice {
                        bus, device, function,
                        vendor_id: vid,
                        device_id: did,
                        bar0,
                        irq_line: irq,
                    });
                }
            }
        }
    }
    None
}

// разрешить карте писать в память DMA bus mastering
pub fn enable_bus_mastering(dev: &PciDevice) {
    let cmd = config_read_u32(dev.bus, dev.device, dev.function, 0x04);
    // бит 0 IO space enable бит 2 bus master enable
    let new_cmd = cmd | 0x04 | 0x01;
    config_write_u32(dev.bus, dev.device, dev.function, 0x04, new_cmd);
}
