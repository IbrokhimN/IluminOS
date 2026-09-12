use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use smoltcp::socket::icmp;
use smoltcp::time::Instant;
use smoltcp::phy::ChecksumCapabilities;
use alloc::vec;
use crate::tcp::device::NetDevice;
use crate::tcp::rtl8139;
use crate::print_color;
use crate::framebuffer::{GREEN, RED, YELLOW, GRAY};

const OUR_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15); // our address
const GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);  // gateway is qemu itself
const PREFIX: u8 = 24;                                        // /24 mask

// smoltcp needs the current time in ms taken from uptime
fn now() -> Instant {
    Instant::from_millis((crate::time::uptime_secs() as i64) * 1000
        + (crate::time::ticks_since_boot() / 2_500_000) as i64 % 1000)
}

// build smoltcp interface on top of our card
fn build_iface(device: &mut NetDevice, mac: [u8; 6]) -> Interface {
    // config with our card mac address
    let config = Config::new(EthernetAddress(mac).into());
    let mut iface = Interface::new(config, device, now());

    // set our ip address on the interface
    iface.update_ip_addrs(|addrs| {
        addrs.push(IpCidr::new(IpAddress::Ipv4(OUR_IP), PREFIX)).ok();
    });
    // default route everything outbound via gateway
    iface.routes_mut().add_default_ipv4_route(GATEWAY).ok();
    iface
}

// send icmp echo to target and wait for reply
pub fn cmd_ping(target: &str) {
    // parse ip from string
    let ip = match parse_ipv4(target) {
        Some(o) => Ipv4Address::new(o[0], o[1], o[2], o[3]),
        None => { print_color!(RED, "bad ip: {}\n", target); return; }
    };

    // bring up card if needed and get its mac
    if !rtl8139::init() {
        print_color!(RED, "no network card\n");
        return;
    }
    let mac = match rtl8139::mac_address() {
        Some(m) => m,
        None => { print_color!(RED, "no MAC\n"); return; }
    };

    // build device and smoltcp interface
    let mut device = NetDevice::new();
    let mut iface = build_iface(&mut device, mac);

    // icmp socket rx and tx buffers
    let rx_buf = icmp::PacketBuffer::new(
        vec![icmp::PacketMetadata::EMPTY; 8],
        vec![0; 256],
    );
    let tx_buf = icmp::PacketBuffer::new(
        vec![icmp::PacketMetadata::EMPTY; 8],
        vec![0; 256],
    );
    let icmp_socket = icmp::Socket::new(rx_buf, tx_buf);

    // add socket to the socket set
    let mut sockets = SocketSet::new(vec![]);
    let handle = sockets.add(icmp_socket);

    // our ping identifier to tell our replies from others
    let ident = 0x22b;
    // bind socket to this identifier
    {
        let socket = sockets.get_mut::<icmp::Socket>(handle);
        // bind by ident to catch only our echoes
        if socket.bind(icmp::Endpoint::Ident(ident)).is_err() {
            print_color!(RED, "icmp bind failed\n");
            return;
        }
    }

    print_color!(YELLOW, "PING {}.{}.{}.{}\n", ip.0[0], ip.0[1], ip.0[2], ip.0[3]);

    let seq: u16 = 0;         // echo sequence number
    let mut sent = false;         // whether request was sent
    let mut waited: u32 = 0;      // reply wait counter
    let mut got_reply = false;

    // main loop poll send echo wait for reply up to a limit
    for _ in 0..2_000_000u32 {
        // pump the stack receive process transmit
        iface.poll(now(), &mut device, &mut sockets);

        let socket = sockets.get_mut::<icmp::Socket>(handle);

        // send echo request once socket is ready
        if !sent && socket.can_send() {
            // build icmp echo request manually
            let icmp_repr = smoltcp::wire::Icmpv4Repr::EchoRequest {
                ident,
                seq_no: seq,
                data: b"iluminos",   // arbitrary echo payload
            };
            // allocate space in the socket and write the packet
            if let Ok(payload) = socket.send(icmp_repr.buffer_len(), ip.into()) {
                let mut packet = smoltcp::wire::Icmpv4Packet::new_unchecked(payload);
                icmp_repr.emit(&mut packet, &ChecksumCapabilities::default());
                sent = true;
            }
        }

        // check for and read a reply
        if socket.can_recv() {
            if let Ok((payload, _addr)) = socket.recv() {
                // parse as an icmp packet
                if let Ok(packet) = smoltcp::wire::Icmpv4Packet::new_checked(payload) {
                    if let Ok(repr) = smoltcp::wire::Icmpv4Repr::parse(
                        &packet, &ChecksumCapabilities::default()
                    ) {
                        // echo reply on our ident means host is alive
                        if let smoltcp::wire::Icmpv4Repr::EchoReply { .. } = repr {
                            print_color!(GREEN, "reply from {}.{}.{}.{}  seq={}\n",
                                ip.0[0], ip.0[1], ip.0[2], ip.0[3], seq);
                            got_reply = true;
                            break;
                        }
                    }
                }
            }
        }

        // reply timeout countdown
        if sent {
            waited += 1;
            if waited > 1_500_000 {
                break; // waited too long treat as no reply
            }
        }
        core::hint::spin_loop();
    }

    let _ = seq; // seq fixed at 0 for now
    if !got_reply {
        print_color!(GRAY, "no reply (timeout)\n");
        // no reply from gateway means driver issue no reply beyond means nat or dns
    }
}

// simple manual ipv4 string parser
fn parse_ipv4(s: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut idx = 0;
    for part in s.split('.') {
        if idx >= 4 || part.is_empty() { return None; }
        let mut n: u32 = 0;
        for b in part.bytes() {
            if b < b'0' || b > b'9' { return None; }
            n = n * 10 + (b - b'0') as u32;
            if n > 255 { return None; }
        }
        octets[idx] = n as u8;
        idx += 1;
    }
    if idx == 4 { Some(octets) } else { None }
}
