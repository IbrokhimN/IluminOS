// прослойка между драйвером RTL8139 и smoltcp через трейт Device
// сигнатуры под smoltcp 0.11.x

use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;
use crate::tcp::rtl8139;

const MTU: usize = 1500; // макс размер кадра Ethernet

// карта глазами smoltcp пустышка вся работа в драйвере
pub struct NetDevice;

impl NetDevice {
    pub fn new() -> Self {
        NetDevice
    }
}

// реализация Device подключает карту к smoltcp
impl Device for NetDevice {
    type RxToken<'a> = NetRxToken;
    type TxToken<'a> = NetTxToken;

    // есть ли принятый кадр
    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut buf = [0u8; MTU];
        match rtl8139::receive(&mut buf) {
            Some(len) => {
                let mut data = [0u8; MTU];
                data[..len].copy_from_slice(&buf[..len]);
                Some((NetRxToken { data, len }, NetTxToken))
            }
            None => None,
        }
    }

    // smoltcp хочет отправить даём TX токен
    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(NetTxToken)
    }

    // возможности карты
    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = MTU;
        caps.medium = Medium::Ethernet;
        caps
    }
}

// держит принятые данные пока smoltcp их не потребит
pub struct NetRxToken {
    data: [u8; MTU],
    len: usize,
}

impl RxToken for NetRxToken {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(&mut self.data[..self.len])
    }
}

// smoltcp заполняет буфер а мы шлём через драйвер
pub struct NetTxToken;

impl TxToken for NetTxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buf = [0u8; MTU];
        let result = f(&mut buf[..len]);
        rtl8139::send(&buf[..len]);
        result
    }
}
