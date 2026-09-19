// gemini client
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};

use crate::tcp::device::NetDevice;
use crate::tcp::dns;
use crate::tcp::rtl8139;

const OUR_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);
const GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
const PREFIX: u8 = 24;
pub const DEFAULT_PORT: u16 = 1965;

const CONNECT_SPIN_LIMIT: u32 = 3_000_000;
const IO_SPIN_LIMIT: u32 = 6_000_000;

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

#[derive(Debug, Clone, Copy)]
pub enum GeminiError {
    NoNic,
    BadUrl,
    DnsFailed,
    Connect,
    Timeout,
    Tls(&'static str),
    BadResponse,
}

// url parsing

#[derive(Clone)]
pub struct GeminiUrl {
    pub host: String,
    pub port: u16,
    pub resource: String, // path and query
}

pub fn parse_url(input: &str) -> Result<GeminiUrl, GeminiError> {
    let s = input.trim();
    let s = s.strip_prefix("gemini://").unwrap_or(s);
    if s.is_empty() {
        return Err(GeminiError::BadUrl);
    }
    let (authority, rest) = match s.find('/') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    if authority.is_empty() {
        return Err(GeminiError::BadUrl);
    }
    let (host, port) = match authority.rfind(':') {
        Some(i) => (
            &authority[..i],
            authority[i + 1..].parse::<u16>().unwrap_or(DEFAULT_PORT),
        ),
        None => (authority, DEFAULT_PORT),
    };
    let resource = if rest.is_empty() { String::from("/") } else { String::from(rest) };
    Ok(GeminiUrl { host: String::from(host), port, resource })
}

pub fn url_string(u: &GeminiUrl) -> String {
    if u.port == DEFAULT_PORT {
        alloc::format!("gemini://{}{}", u.host, u.resource)
    } else {
        alloc::format!("gemini://{}:{}{}", u.host, u.port, u.resource)
    }
}

// raw tcp transport

// blocking tcp stream
pub struct TcpStream {
    device: NetDevice,
    iface: Interface,
    sockets: SocketSet<'static>,
    handle: SocketHandle,
}

impl TcpStream {
    pub fn connect(host: &str, port: u16) -> Result<Self, GeminiError> {
        if !rtl8139::init() {
            return Err(GeminiError::NoNic);
        }
        let mac = rtl8139::mac_address().ok_or(GeminiError::NoNic)?;
        let ip = dns::resolve(host).ok_or(GeminiError::DnsFailed)?;

        let mut device = NetDevice::new();
        let mut iface = build_iface(&mut device, mac);

        let rx_buf = tcp::SocketBuffer::new(vec![0u8; 8192]);
        let tx_buf = tcp::SocketBuffer::new(vec![0u8; 2048]);
        let mut socket = tcp::Socket::new(rx_buf, tx_buf);

        let local_port = 40000u16.wrapping_add(crate::random::next_range(20000) as u16);
        {
            let cx = iface.context();
            socket
                .connect(cx, (IpAddress::Ipv4(ip), port), local_port)
                .map_err(|_| GeminiError::Connect)?;
        }

        let mut sockets = SocketSet::new(Vec::new());
        let handle = sockets.add(socket);

        let mut spins = 0u32;
        loop {
            iface.poll(now(), &mut device, &mut sockets);
            let sock = sockets.get::<tcp::Socket>(handle);
            if sock.state() == tcp::State::Established {
                break;
            }
            if sock.state() == tcp::State::Closed {
                return Err(GeminiError::Connect);
            }
            spins += 1;
            if spins > CONNECT_SPIN_LIMIT {
                return Err(GeminiError::Timeout);
            }
            core::hint::spin_loop();
        }

        Ok(TcpStream { device, iface, sockets, handle })
    }

    pub fn write_all(&mut self, mut data: &[u8]) -> Result<(), GeminiError> {
        let mut spins = 0u32;
        while !data.is_empty() {
            self.iface.poll(now(), &mut self.device, &mut self.sockets);
            let sock = self.sockets.get_mut::<tcp::Socket>(self.handle);
            if sock.can_send() {
                if let Ok(n) = sock.send_slice(data) {
                    if n > 0 {
                        data = &data[n..];
                        spins = 0;
                        continue;
                    }
                }
            }
            spins += 1;
            if spins > IO_SPIN_LIMIT {
                return Err(GeminiError::Timeout);
            }
            core::hint::spin_loop();
        }
        Ok(())
    }

    // read until close
    pub fn read_to_end(&mut self, out: &mut Vec<u8>) -> Result<(), GeminiError> {
        let mut spins = 0u32;
        let mut buf = [0u8; 1024];
        loop {
            self.iface.poll(now(), &mut self.device, &mut self.sockets);
            let sock = self.sockets.get_mut::<tcp::Socket>(self.handle);

            if sock.can_recv() {
                if let Ok(n) = sock.recv_slice(&mut buf) {
                    if n > 0 {
                        out.extend_from_slice(&buf[..n]);
                        spins = 0;
                        continue;
                    }
                }
            }

            if !sock.may_recv() {
                break; // closed
            }

            spins += 1;
            if spins > IO_SPIN_LIMIT {
                return Err(GeminiError::Timeout);
            }
            core::hint::spin_loop();
        }
        Ok(())
    }
}

// tls module

pub mod tls {
    use super::{GeminiError, TcpStream, IO_SPIN_LIMIT};
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_io::{ErrorKind, ErrorType, Read as EioRead, Write as EioWrite};
    use embedded_tls::blocking::{Aes128GcmSha256, NoVerify, TlsConfig, TlsConnection, TlsContext};
    use rand_core::{CryptoRng, RngCore};

    // toggle tls
    pub const USE_TLS: bool = true;

    // embedded io glue

    #[derive(Debug)]
    pub struct IoError(pub GeminiError);

    impl embedded_io::Error for IoError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    impl ErrorType for TcpStream {
        type Error = IoError;
    }

    impl EioRead for TcpStream {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let mut spins = 0u32;
            loop {
                self.iface.poll(super::now(), &mut self.device, &mut self.sockets);
                let sock = self.sockets.get_mut::<super::tcp::Socket>(self.handle);

                if sock.can_recv() {
                    match sock.recv_slice(buf) {
                        Ok(n) => return Ok(n),
                        Err(_) => return Err(IoError(GeminiError::Tls("tcp recv error"))),
                    }
                }
                if !sock.may_recv() {
                    return Ok(0); // eof
                }
                spins += 1;
                if spins > IO_SPIN_LIMIT {
                    return Err(IoError(GeminiError::Timeout));
                }
                core::hint::spin_loop();
            }
        }
    }

    impl EioWrite for TcpStream {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            let mut spins = 0u32;
            loop {
                self.iface.poll(super::now(), &mut self.device, &mut self.sockets);
                let sock = self.sockets.get_mut::<super::tcp::Socket>(self.handle);

                if sock.can_send() {
                    match sock.send_slice(buf) {
                        Ok(n) if n > 0 => return Ok(n),
                        Ok(_) => {}
                        Err(_) => return Err(IoError(GeminiError::Tls("tcp send error"))),
                    }
                }
                spins += 1;
                if spins > IO_SPIN_LIMIT {
                    return Err(IoError(GeminiError::Timeout));
                }
                core::hint::spin_loop();
            }
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    // rng glue
    struct KernelRng;

    impl RngCore for KernelRng {
        fn next_u32(&mut self) -> u32 {
            crate::random::next_u64() as u32
        }
        fn next_u64(&mut self) -> u64 {
            crate::random::next_u64()
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            let mut i = 0;
            while i < dest.len() {
                let chunk = crate::random::next_u64().to_le_bytes();
                let n = core::cmp::min(8, dest.len() - i);
                dest[i..i + n].copy_from_slice(&chunk[..n]);
                i += n;
            }
        }
        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }
    impl CryptoRng for KernelRng {}

    // tls request
    pub fn request(inner: TcpStream, sni: &str, request_line: &[u8]) -> Result<Vec<u8>, GeminiError> {
        let mut read_buf = vec![0u8; 16640];
        let mut write_buf = vec![0u8; 16640];

        let config = TlsConfig::new().with_server_name(sni);
        let mut rng = KernelRng;
        let mut tls_conn: TlsConnection<'_, TcpStream, Aes128GcmSha256> =
            TlsConnection::new(inner, &mut read_buf[..], &mut write_buf[..]);

        tls_conn
            .open::<KernelRng, NoVerify>(TlsContext::new(&config, &mut rng))
            .map_err(|_| GeminiError::Tls("handshake failed"))?;

        write_all_tls(&mut tls_conn, request_line)?;
        EioWrite::flush(&mut tls_conn).map_err(|_| GeminiError::Tls("tls flush failed"))?;

        let mut out = Vec::new();
        let mut buf = [0u8; 2048];
        loop {
            match EioRead::read(&mut tls_conn, &mut buf) {
                Ok(0) => break,
                Ok(n) => out.extend_from_slice(&buf[..n]),
                Err(_) => break, // close
            }
        }

        Ok(out)
    }

    fn write_all_tls<S, C>(conn: &mut TlsConnection<'_, S, C>, mut data: &[u8]) -> Result<(), GeminiError>
    where
        S: embedded_io::Read + embedded_io::Write,
        C: embedded_tls::blocking::TlsCipherSuite + 'static,
    {
        while !data.is_empty() {
            match EioWrite::write(conn, data) {
                Ok(0) => return Err(GeminiError::Tls("tls write stalled")),
                Ok(n) => data = &data[n..],
                Err(_) => return Err(GeminiError::Tls("tls write error")),
            }
        }
        Ok(())
    }

    // trust store
    pub mod trust_store {
        use alloc::string::String;
        use alloc::vec::Vec;
        use spin::Mutex;

        static KNOWN: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

        pub fn get(host: &str) -> Option<String> {
            KNOWN.lock().iter().find(|(h, _)| h == host).map(|(_, fp)| fp.clone())
        }

        pub fn remember(host: &str, fingerprint: &str) {
            let mut g = KNOWN.lock();
            if let Some(entry) = g.iter_mut().find(|(h, _)| h == host) {
                entry.1 = String::from(fingerprint);
            } else {
                g.push((String::from(host), String::from(fingerprint)));
            }
        }
    }
}

// protocol handling

pub struct GeminiResponse {
    pub status: u8,
    pub meta: String,
    pub body: Vec<u8>,
}

impl GeminiResponse {
    pub fn category(&self) -> u8 {
        self.status / 10
    }
}

// fetch url
pub fn fetch(url_str: &str) -> Result<(GeminiUrl, GeminiResponse), GeminiError> {
    let url = parse_url(url_str)?;
    let tcp_stream = TcpStream::connect(&url.host, url.port)?;

    let request = alloc::format!("{}\r\n", url_string(&url));

    let raw = if tls::USE_TLS {
        tls::request(tcp_stream, &url.host, request.as_bytes())?
    } else {
        let mut stream = tcp_stream;
        stream.write_all(request.as_bytes())?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;
        buf
    };

    let response = parse_response(&raw)?;
    Ok((url, response))
}

fn parse_response(raw: &[u8]) -> Result<GeminiResponse, GeminiError> {
    let nl = raw.iter().position(|&b| b == b'\n').ok_or(GeminiError::BadResponse)?;
    let header_bytes = &raw[..nl];
    let header_bytes = header_bytes.strip_suffix(b"\r").unwrap_or(header_bytes);
    let header = core::str::from_utf8(header_bytes).map_err(|_| GeminiError::BadResponse)?;

    if header.len() < 2 {
        return Err(GeminiError::BadResponse);
    }
    let status: u8 = header[..2].parse().map_err(|_| GeminiError::BadResponse)?;
    let meta = header.get(2..).unwrap_or("").trim_start().to_string();
    let body = raw[nl + 1..].to_vec();

    Ok(GeminiResponse { status, meta, body })
}
