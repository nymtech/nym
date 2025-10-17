use rustls::{pki_types::ServerName, ClientConfig, ClientConnection};
use smolmix::{create_device, SmolmixError};
use std::{
    io::{self, Read, Write},
    sync::Arc,
};
use tracing::info;

use nym_sdk::stream_wrapper::IpMixStream;
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp,
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
use std::sync::Once;
use std::time::Duration;

static INIT: Once = Once::new();

pub struct TlsOverTcp {
    pub conn: ClientConnection,
}

impl TlsOverTcp {
    pub fn new(domain: &str) -> Result<Self, SmolmixError> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let server_name = ServerName::try_from(domain)
            .map_err(|_| SmolmixError::InvalidDnsName)?
            .to_owned();

        let conn = ClientConnection::new(Arc::new(config), server_name)
            .map_err(|_| SmolmixError::TlsHandshakeFailed)?;

        Ok(Self { conn })
    }

    /// Move data from TLS connection to TCP socket
    pub fn write_tls(&mut self, socket: &mut tcp::Socket) -> Result<(), SmolmixError> {
        let mut buf = [0u8; 4096];
        while self.conn.wants_write() {
            match self.conn.write_tls(&mut buf.as_mut_slice()) {
                Ok(n) if n > 0 => {
                    socket
                        .send_slice(&buf[..n])
                        .map_err(|_| SmolmixError::TlsHandshakeFailed)?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Move data from TCP socket to TLS connection
    pub fn read_tls(&mut self, socket: &mut tcp::Socket) -> Result<(), SmolmixError> {
        if socket.can_recv() {
            let _ = socket.recv(|chunk| {
                if !chunk.is_empty() {
                    inspect_tls_packet(chunk);
                    let _ = self.conn.read_tls(&mut io::Cursor::new(&mut *chunk));
                    let _ = self.conn.process_new_packets();
                }
                (chunk.len(), ())
            });
        }
        Ok(())
    }

    pub fn send(&mut self, data: &[u8], socket: &mut tcp::Socket) -> Result<(), SmolmixError> {
        self.conn
            .writer()
            .write_all(data)
            .map_err(|_| SmolmixError::TlsHandshakeFailed)?;
        self.write_tls(socket)
    }

    pub fn recv(&mut self, socket: &mut tcp::Socket) -> Result<Vec<u8>, SmolmixError> {
        self.read_tls(socket)?;
        let mut result = Vec::new();
        let mut buf = vec![0u8; 4096];
        match self.conn.reader().read(&mut buf) {
            Ok(n) if n > 0 => result.extend_from_slice(&buf[..n]),
            _ => {}
        }
        Ok(result)
    }
}

fn inspect_tls_packet(data: &[u8]) {
    if data.len() < 5 {
        return;
    }
    let content_type = data[0];
    if content_type < 0x14 || content_type > 0x17 {
        return;
    }
    let version = u16::from_be_bytes([data[1], data[2]]);
    let length = u16::from_be_bytes([data[3], data[4]]);
    info!(
        "TLS packet: ContentType={:#04x}, Version={:#06x}, Length={}",
        content_type, version, length
    );
    if content_type == 0x16 && data.len() > 5 {
        let handshake_type = data[5];
        let handshake_types = match handshake_type {
            0x01 => "ClientHello",
            0x02 => "ServerHello",
            0x0b => "Certificate",
            0x0c => "ServerKeyExchange",
            0x0d => "CertificateRequest",
            0x0e => "ServerHelloDone",
            0x0f => "CertificateVerify",
            0x10 => "ClientKeyExchange",
            0x14 => "Finished",
            _ => "Unknown",
        };
        info!(
            "Handshake type: {:#04x} ({}), Length: {}",
            handshake_type, handshake_types, length
        );
    }
}

fn init_logging() {
    if tracing::dispatcher::has_been_set() {
        return;
    }
    INIT.call_once(|| {
        nym_bin_common::logging::setup_tracing_logger();
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let ipr_stream = IpMixStream::new().await?;
    let (mut device, bridge, allocated_ips) = create_device(ipr_stream).await?;
    info!("Allocated IP: {}", allocated_ips.ipv4);

    tokio::spawn(async move {
        bridge.run().await.unwrap();
    });

    let config = Config::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, &mut device, Instant::now());
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::new(IpAddress::from(allocated_ips.ipv4), 32))
            .unwrap();
    });
    iface
        .routes_mut()
        .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
        .unwrap();

    let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 16384]);
    let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    let mut sockets = SocketSet::new(vec![]);
    let tcp_handle = sockets.add(tcp_socket);

    let target_ip = Ipv4Address::new(1, 1, 1, 1); // Pinging Cloudflare
    let target_port = 443;
    info!("Connecting to {}:{} through mixnet", target_ip, target_port);

    let mut timestamp = Instant::from_millis(0);
    let start = tokio::time::Instant::now();
    let mut connected = false;
    let mut tls = None;
    let mut handshake_completed = false;

    loop {
        if start.elapsed() > Duration::from_secs(120) {
            info!("Test timeout after 120 seconds");
            break;
        }

        iface.poll(timestamp, &mut device, &mut sockets);
        timestamp += smoltcp::time::Duration::from_millis(1);
        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);

        if !connected && !socket.is_open() {
            match socket.connect(iface.context(), (target_ip, target_port), 49152) {
                Ok(_) => {
                    info!("TCP connect started");
                    connected = true;
                }
                Err(e) => {
                    info!("TCP connect failed: {}", e);
                    break;
                }
            }
        }

        if start.elapsed().as_secs() % 5 == 0 && start.elapsed().as_millis() % 1000 < 100 {
            info!(
                "State: TCP={:?}, established={}, can_send={}, can_recv={}",
                socket.state(),
                socket.state() == tcp::State::Established,
                socket.may_send(),
                socket.can_recv()
            );
        }

        if socket.state() == tcp::State::Established && tls.is_none() {
            info!("TCP established - creating TLS connection");
            match TlsOverTcp::new("cloudflare.com") {
                Ok(t) => tls = Some(t),
                Err(e) => {
                    info!("TLS create failed: {}", e);
                    break;
                }
            }
        }

        if let Some(ref mut tls_conn) = tls {
            let _ = tls_conn.read_tls(socket);
            let _ = tls_conn.write_tls(socket);

            if start.elapsed().as_secs() % 10 == 0 && start.elapsed().as_millis() % 1000 < 100 {
                info!(
                    "TLS state: handshaking={}, wants_read={}, wants_write={}",
                    tls_conn.conn.is_handshaking(),
                    tls_conn.conn.wants_read(),
                    tls_conn.conn.wants_write()
                );
            }

            if !tls_conn.conn.is_handshaking() && !handshake_completed {
                info!("TLS handshake complete");
                info!(
                    "TLS verification: handshake_complete=true, wants_read={}, wants_write={}",
                    tls_conn.conn.wants_read(),
                    tls_conn.conn.wants_write()
                );

                match tls_conn.recv(socket) {
                    Ok(data) if data.is_empty() => {
                        info!("No unexpected application data waiting to be read");
                    }
                    Ok(data) => {
                        info!("Unexpected application data received: {} bytes", data.len());
                    }
                    Err(e) => {
                        info!("TLS recv check failed: {}", e);
                    }
                }
                info!("TLS handshake successful with cloudflare");
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    info!("Test completed");
    Ok(())
}
