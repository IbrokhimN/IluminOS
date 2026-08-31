// сетевая подсистема pci + rtl8139 + device + net smoltcp
// ping работает опросом без прерываний

pub mod pci;        // сканер PCI
pub mod rtl8139;    // драйвер карты опрос без IRQ
pub mod device;     // прослойка к smoltcp
pub mod net;        // стек smoltcp + ping

// pub mod interrupts; // прерывания на потом
