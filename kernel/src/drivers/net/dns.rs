use alloc::vec;
use alloc::vec::Vec;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::socket::udp;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};

use crate::tcp::device::NetDevice;
use crate::tcp::rtl8139;

const OUR_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);
const GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
const PREFIX: u8 = 24;
// qemu slirp dns
const RESOLVER: Ipv4Address = Ipv4Address::new(10, 0, 2, 3);
const SPIN_LIMIT: u32 = 2_000_000;

fn now() -> Instant {
    Instant::from_millis(
        (crate::time::uptime_secs() as i64) * 1000
            + (crate::time::ticks_since_boot() / 2_500_000) as i64 % 1000,
    )
}

fn build_iface(device: &mut NetDevice, mac: [u8; 6]) -> Interface {
    let config = Config::new(EthernetAddress(mac).into());
    let mut iface = Interface::new(config, device, now());
    iface.update_ip_addrs(|addrs| {
        addrs.push(IpCidr::new(IpAddress::Ipv4(OUR_IP), PREFIX)).ok();
    });
    iface.routes_mut().add_default_ipv4_route(GATEWAY).ok();
    iface
}

// resolve hostname
pub fn resolve(host: &str) -> Option<Ipv4Address> {
    if let Some(ip) = parse_ipv4_literal(host) {
        return Some(ip);
    }

    if !rtl8139::init() {
        return None;
    }
    let mac = rtl8139::mac_address()?;

    let mut device = NetDevice::new();
    let mut iface = build_iface(&mut device, mac);

    let rx_buf = udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 4], vec![0; 512]);
    let tx_buf = udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 4], vec![0; 512]);
    let mut socket = udp::Socket::new(rx_buf, tx_buf);
    let local_port = 40000u16.wrapping_add(crate::random::next_range(20000) as u16);
    if socket.bind(local_port).is_err() {
        return None;
    }

    let mut sockets = SocketSet::new(Vec::new());
    let handle = sockets.add(socket);

    let query = build_query(host);
    let mut sent = false;

    for _ in 0..SPIN_LIMIT {
        iface.poll(now(), &mut device, &mut sockets);
        let socket = sockets.get_mut::<udp::Socket>(handle);

        if !sent && socket.can_send() {
            if socket.send_slice(&query, (IpAddress::Ipv4(RESOLVER), 53u16)).is_ok() {
                sent = true;
            }
        }

        if socket.can_recv() {
            if let Ok((data, _meta)) = socket.recv() {
                if let Some(ip) = parse_response(data) {
                    return Some(ip);
                }
            }
        }

        core::hint::spin_loop();
    }

    None
}

fn parse_ipv4_literal(s: &str) -> Option<Ipv4Address> {
    let mut octets = [0u8; 4];
    let mut idx = 0;
    for part in s.split('.') {
        if idx >= 4 || part.is_empty() {
            return None;
        }
        let mut n: u32 = 0;
        for b in part.bytes() {
            if !b.is_ascii_digit() {
                return None;
            }
            n = n * 10 + (b - b'0') as u32;
            if n > 255 {
                return None;
            }
        }
        octets[idx] = n as u8;
        idx += 1;
    }
    if idx == 4 {
        Some(Ipv4Address::new(octets[0], octets[1], octets[2], octets[3]))
    } else {
        None
    }
}

// build query
fn build_query(host: &str) -> Vec<u8> {
    let mut q = Vec::with_capacity(host.len() + 16);
    let id = (crate::random::next_range(0xFFFF) as u16).to_be_bytes();
    q.extend_from_slice(&id);
    q.extend_from_slice(&[0x01, 0x00]); // flags
    q.extend_from_slice(&[0x00, 0x01]); // qdcount
    q.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // counts
    for label in host.split('.') {
        q.push(label.len() as u8);
        q.extend_from_slice(label.as_bytes());
    }
    q.push(0);
    q.extend_from_slice(&[0x00, 0x01]); // qtype a
    q.extend_from_slice(&[0x00, 0x01]); // qclass in
    q
}

// parse response
fn parse_response(data: &[u8]) -> Option<Ipv4Address> {
    if data.len() < 12 {
        return None;
    }
    let ancount = u16::from_be_bytes([data[6], data[7]]);
    if ancount == 0 {
        return None;
    }

    let mut pos = skip_name(data, 12)?;
    pos += 4; // qtype and qclass

    for _ in 0..ancount {
        pos = skip_name(data, pos)?;
        if pos + 10 > data.len() {
            return None;
        }
        let rtype = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let rdlen = u16::from_be_bytes([data[pos + 8], data[pos + 9]]) as usize;
        pos += 10;
        if pos + rdlen > data.len() {
            return None;
        }
        if rtype == 1 && rdlen == 4 {
            return Some(Ipv4Address::new(data[pos], data[pos + 1], data[pos + 2], data[pos + 3]));
        }
        pos += rdlen;
    }
    None
}

// skip name
fn skip_name(data: &[u8], mut pos: usize) -> Option<usize> {
    loop {
        if pos >= data.len() {
            return None;
        }
        let len = data[pos];
        if len == 0 {
            return Some(pos + 1);
        }
        if len & 0xC0 == 0xC0 {
            return Some(pos + 2); // pointer offset
        }
        pos += 1 + len as usize;
    }
}
