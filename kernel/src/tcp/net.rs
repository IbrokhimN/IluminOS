// сеть на smoltcp сборка стека + команда ping ICMP

use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use smoltcp::socket::icmp;
use smoltcp::time::Instant;
use smoltcp::phy::ChecksumCapabilities;
use alloc::vec;
use crate::tcp::device::NetDevice;
use crate::tcp::rtl8139;
use crate::{print, println, print_color};
use crate::framebuffer::{GREEN, RED, YELLOW, GRAY};

// адреса для QEMU user mode не менять
const OUR_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15); // наш адрес
const GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);  // шлюз сам QEMU
const PREFIX: u8 = 24;                                        // маска /24

// smoltcp нужен текущий момент в мс берём из uptime
fn now() -> Instant {
    Instant::from_millis((crate::time::uptime_secs() as i64) * 1000
        + (crate::time::ticks_since_boot() / 2_500_000) as i64 % 1000)
}

// собрать интерфейс smoltcp поверх карты
fn build_iface(device: &mut NetDevice, mac: [u8; 6]) -> Interface {
    // конфиг наш MAC
    let config = Config::new(EthernetAddress(mac).into());
    let mut iface = Interface::new(config, device, now());

    // прописать наш IP
    iface.update_ip_addrs(|addrs| {
        addrs.push(IpCidr::new(IpAddress::Ipv4(OUR_IP), PREFIX)).ok();
    });
    // маршрут по умолчанию через шлюз
    iface.routes_mut().add_default_ipv4_route(GATEWAY).ok();
    iface
}

// послать ICMP echo на target и подождать ответ
pub fn cmd_ping(target: &str) {
    // разобрать IP из строки
    let ip = match parse_ipv4(target) {
        Some(o) => Ipv4Address::new(o[0], o[1], o[2], o[3]),
        None => { print_color!(RED, "bad ip: {}\n", target); return; }
    };

    // поднять карту и взять MAC
    if !rtl8139::init() {
        print_color!(RED, "no network card\n");
        return;
    }
    let mac = match rtl8139::mac_address() {
        Some(m) => m,
        None => { print_color!(RED, "no MAC\n"); return; }
    };

    // собрать устройство + интерфейс
    let mut device = NetDevice::new();
    let mut iface = build_iface(&mut device, mac);

    // ICMP сокет буферы приёма/передачи
    let rx_buf = icmp::PacketBuffer::new(
        vec![icmp::PacketMetadata::EMPTY; 8],
        vec![0; 256],
    );
    let tx_buf = icmp::PacketBuffer::new(
        vec![icmp::PacketMetadata::EMPTY; 8],
        vec![0; 256],
    );
    let icmp_socket = icmp::Socket::new(rx_buf, tx_buf);

    // положить сокет в набор
    let mut sockets = SocketSet::new(vec![]);
    let handle = sockets.add(icmp_socket);

    // номер нашего ping чтобы отличить свой ответ
    let ident = 0x22b;
    {
        let socket = sockets.get_mut::<icmp::Socket>(handle);
        // ловим ответы именно на наши echo
        if socket.bind(icmp::Endpoint::Ident(ident)).is_err() {
            print_color!(RED, "icmp bind failed\n");
            return;
        }
    }

    print_color!(YELLOW, "PING {}.{}.{}.{}\n", ip.0[0], ip.0[1], ip.0[2], ip.0[3]);

    let mut seq: u16 = 0;         // номер echo пакета
    let mut sent = false;         // отправили ли запрос
    let mut waited: u32 = 0;      // счётчик ожидания
    let mut got_reply = false;

    // главный цикл крутим poll шлём echo ждём reply
    for _ in 0..2_000_000u32 {
        // прокрутить стек приём обработка передача
        iface.poll(now(), &mut device, &mut sockets);

        let socket = sockets.get_mut::<icmp::Socket>(handle);

        // если можем слать и ещё не послали шлём echo request
        if !sent && socket.can_send() {
            let icmp_repr = smoltcp::wire::Icmpv4Repr::EchoRequest {
                ident,
                seq_no: seq,
                data: b"iluminos",
            };
            if let Ok(payload) = socket.send(icmp_repr.buffer_len(), ip.into()) {
                let mut packet = smoltcp::wire::Icmpv4Packet::new_unchecked(payload);
                icmp_repr.emit(&mut packet, &ChecksumCapabilities::default());
                sent = true;
            }
        }

        // если пришёл ответ прочитать и проверить
        if socket.can_recv() {
            if let Ok((payload, _addr)) = socket.recv() {
                if let Ok(packet) = smoltcp::wire::Icmpv4Packet::new_checked(payload) {
                    if let Ok(repr) = smoltcp::wire::Icmpv4Repr::parse(
                        &packet, &ChecksumCapabilities::default()
                    ) {
                        // echo reply на наш ident значит хост живой
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

        // отсчёт таймаута
        if sent {
            waited += 1;
            if waited > 1_500_000 {
                break;
            }
        }
        core::hint::spin_loop();
    }

    let _ = seq;
    if !got_reply {
        print_color!(GRAY, "no reply (timeout)\n");
    }
}

// парсер 10.0.2.2 -> [10 0 2 2]
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
