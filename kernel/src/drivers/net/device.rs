use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;
use crate::tcp::rtl8139;

const MTU: usize = 1500; // max ethernet frame size

// smoltcp view of the card actual work is in the driver
pub struct NetDevice;

impl NetDevice {
    pub fn new() -> Self {
        NetDevice
    }
}

// device impl plugs the card into smoltcp
impl Device for NetDevice {
    type RxToken<'a> = NetRxToken;
    type TxToken<'a> = NetTxToken;

    // check if a frame was received
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

    // smoltcp wants to send give it a tx token
    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(NetTxToken)
    }

    // card capabilities
    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = MTU;
        caps.medium = Medium::Ethernet;
        caps
    }
}

// holds received data until smoltcp consumes it
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

// smoltcp fills the buffer then we send via driver
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
