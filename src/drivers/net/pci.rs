// pci bus scanner finds devices on the board

use crate::port::{inl, outl};

// rtl8139 identified by this vendor device pair
pub const RTL8139_VENDOR: u16 = 0x10EC; // Realtek
pub const RTL8139_DEVICE: u16 = 0x8139;

// a found pci device
#[derive(Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub bar0: u32,      // base address
    pub irq_line: u8,   // irq number
}

// build 32 bit address for port 0xcf8
fn make_address(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    (1 << 31)                        // enable bit
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC)   // align to 4 bytes
}

// read 32 bit field from device config
pub fn config_read_u32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let addr = make_address(bus, device, function, offset);
    outl(0xCF8, addr);  // write address
    inl(0xCFC)          // read data
}

// read 16 bit field half of u32
pub fn config_read_u16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let dword = config_read_u32(bus, device, function, offset);
    // offset bit 2 picks which half
    ((dword >> ((offset as u32 & 2) * 8)) & 0xFFFF) as u16
}

// write 32 bit field
pub fn config_write_u32(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let addr = make_address(bus, device, function, offset);
    outl(0xCF8, addr);
    outl(0xCFC, value);
}

// scan whole bus for device by vendor and device id
pub fn find_device(vendor: u16, device_id_wanted: u16) -> Option<PciDevice> {
    // scan all 256 buses 32 devices 8 functions
    for bus in 0..256u16 {
        for device in 0..32u8 {
            for function in 0..8u8 {
                let bus = bus as u8;
                // read vendor id
                let vid = config_read_u16(bus, device, function, 0x00);
                // 0xffff means empty slot
                if vid == 0xFFFF {
                    continue;
                }
                let did = config_read_u16(bus, device, function, 0x02);

                // this is the one we want
                if vid == vendor && did == device_id_wanted {
                    // bar0 card base address
                    let bar0 = config_read_u32(bus, device, function, 0x10);
                    // irq line low byte
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

// enable dma bus mastering for the card
pub fn enable_bus_mastering(dev: &PciDevice) {
    let cmd = config_read_u32(dev.bus, dev.device, dev.function, 0x04);
    // bit 0 io space enable bit 2 bus master enable
    let new_cmd = cmd | 0x04 | 0x01;
    config_write_u32(dev.bus, dev.device, dev.function, 0x04, new_cmd);
}
