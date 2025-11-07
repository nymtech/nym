#![allow(clippy::result_large_err)]
use mixtcp::{create_device, MixtcpError, NymIprDevice};
use nym_sdk::stream_wrapper::{IpMixStream, NetworkEnvironment};
use reqwest::StatusCode;
use rustls::{pki_types::ServerName, ClientConfig, ClientConnection};
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp,
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
use std::sync::Once;
use std::{
    io::{self, Read, Write},
    sync::Arc,
    time::Duration,
};
use tracing::info;

static INIT: Once = Once::new();

pub struct TlsOverTcp {
    pub conn: ClientConnection,
}

impl TlsOverTcp {
    pub fn new(domain: &str) -> Result<Self, MixtcpError> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let server_name = ServerName::try_from(domain)
            .map_err(|_| MixtcpError::InvalidDnsName)?
            .to_owned();

        let conn = ClientConnection::new(Arc::new(config), server_name)
            .map_err(|_| MixtcpError::TlsHandshakeFailed)?;

        Ok(Self { conn })
    }

    pub fn write_tls(&mut self, socket: &mut tcp::Socket) -> Result<(), MixtcpError> {
        let mut buf = [0u8; 4096];
        while self.conn.wants_write() {
            match self.conn.write_tls(&mut buf.as_mut_slice()) {
                Ok(n) if n > 0 => {
                    socket
                        .send_slice(&buf[..n])
                        .map_err(|_| MixtcpError::TlsHandshakeFailed)?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    pub fn read_tls(&mut self, socket: &mut tcp::Socket) -> Result<(), MixtcpError> {
        if socket.can_recv() {
            let _ = socket.recv(|chunk| {
                if !chunk.is_empty() {
                    let _ = self.conn.read_tls(&mut io::Cursor::new(&mut *chunk));
                    let _ = self.conn.process_new_packets();
                }
                (chunk.len(), ())
            });
        }
        Ok(())
    }

    pub fn send(&mut self, data: &[u8], socket: &mut tcp::Socket) -> Result<(), MixtcpError> {
        self.conn
            .writer()
            .write_all(data)
            .map_err(|_| MixtcpError::TlsHandshakeFailed)?;
        self.write_tls(socket)
    }
}

/// Reqwest-ish client right now, just a handrolled GET request for the example
pub struct MixtcpReqwestClient {
    device: Arc<tokio::sync::Mutex<(smoltcp::iface::Interface, NymIprDevice)>>,
    _bridge: tokio::task::JoinHandle<()>,
    _allocated_ip: Ipv4Address,
}

impl MixtcpReqwestClient {
    pub async fn new() -> Result<Self, MixtcpError> {
        let ipr_stream = IpMixStream::new(NetworkEnvironment::Mainnet)
            .await
            .map_err(|_| MixtcpError::MixnetConnectionFailed)?;

        let (mut device, bridge, allocated_ips) = create_device(ipr_stream).await?;
        info!("Allocated IP: {}", allocated_ips.ipv4);

        let bridge_handle = tokio::spawn(async move {
            if let Err(e) = bridge.run().await {
                tracing::error!("Bridge error: {}", e);
            }
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

        let device = Arc::new(tokio::sync::Mutex::new((iface, device)));

        Ok(Self {
            device,
            _bridge: bridge_handle,
            _allocated_ip: allocated_ips.ipv4,
        })
    }

    pub async fn get(&self, url: &str) -> Result<MixtcpResponse, MixtcpError> {
        let parsed_url = reqwest::Url::parse(url).map_err(|_| MixtcpError::InvalidUrl)?;
        let host = parsed_url.host_str().ok_or(MixtcpError::InvalidUrl)?;
        let path = parsed_url.path();

        let response_bytes = self.simple_get_request(host, path).await?;
        let (status, body) = self.parse_simple_response(&response_bytes)?;

        Ok(MixtcpResponse { status, body })
    }

    async fn simple_get_request(&self, domain: &str, path: &str) -> Result<Vec<u8>, MixtcpError> {
        let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 16384]);
        let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
        let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
        let mut sockets = SocketSet::new(vec![]);
        let tcp_handle = sockets.add(tcp_socket);

        let target_ip = Ipv4Address::new(1, 1, 1, 1);
        let target_port = 443;

        let mut timestamp = Instant::from_millis(0);
        let start = tokio::time::Instant::now();
        let mut connected = false;
        let mut tls = None;
        let mut handshake_completed = false;
        let mut request_sent = false;
        let mut response_data = Vec::new();

        let mut device_guard = self.device.lock().await;
        let (ref mut iface, ref mut device) = &mut *device_guard;

        loop {
            if start.elapsed() > Duration::from_secs(60) {
                return Err(MixtcpError::Timeout);
            }

            iface.poll(timestamp, device, &mut sockets);
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
                        return Err(MixtcpError::TcpConnectionFailed);
                    }
                }
            }

            if socket.state() == tcp::State::Established && tls.is_none() {
                info!("TCP established - creating TLS connection");
                match TlsOverTcp::new(domain) {
                    Ok(t) => tls = Some(t),
                    Err(e) => {
                        info!("TLS create failed: {}", e);
                        return Err(MixtcpError::TlsHandshakeFailed);
                    }
                }
            }

            if let Some(ref mut tls_conn) = tls {
                let _ = tls_conn.read_tls(socket);
                let _ = tls_conn.write_tls(socket);

                if !tls_conn.conn.is_handshaking() && !handshake_completed {
                    handshake_completed = true;
                    info!("TLS handshake completed - ready for HTTPS");

                    let request = format!(
                        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: mixtcp/1.0\r\nAccept: */*\r\nConnection: close\r\n\r\n",
                        path, domain
                    );
                    tls_conn.send(request.as_bytes(), socket)?;
                    info!("HTTPS request sent");
                    request_sent = true;
                }

                if request_sent {
                    let mut buf = vec![0u8; 4096];
                    match tls_conn.conn.reader().read(&mut buf) {
                        Ok(0) => {
                            info!("Response complete");
                            break;
                        }
                        Ok(n) if n > 0 => {
                            response_data.extend_from_slice(&buf[..n]);
                            if let Ok(response_str) = std::str::from_utf8(&response_data) {
                                if response_str.contains("\r\n\r\n") {
                                    return Ok(response_data);
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(e) => {
                            info!("Read error: {}", e);
                            return Err(MixtcpError::ResponseReadFailed);
                        }
                        Ok(_) => continue,
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Err(MixtcpError::NoResponseReceived)
    }

    /// Simple response - just extract status and body
    fn parse_simple_response(&self, response_bytes: &[u8]) -> Result<(u16, String), MixtcpError> {
        let response_str = String::from_utf8_lossy(response_bytes);

        let status_line = response_str
            .lines()
            .next()
            .ok_or(MixtcpError::InvalidHttpResponse)?;

        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(200);

        if let Some(body_start) = response_str.find("\r\n\r\n") {
            let body = response_str[body_start + 4..].to_string();
            Ok((status, body))
        } else {
            Err(MixtcpError::InvalidHttpResponse)
        }
    }
}

pub struct MixtcpResponse {
    status: u16,
    body: String,
}

impl MixtcpResponse {
    pub fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub async fn text(self) -> Result<String, std::convert::Infallible> {
        Ok(self.body)
    }
}

fn init_logging() {
    INIT.call_once_force(|state| {
        if state.is_poisoned() {
            eprintln!("Logger initialization was poisoned, retrying");
        }
        if !tracing::dispatcher::has_been_set() {
            nym_bin_common::logging::setup_tracing_logger();
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let test_url = "https://cloudflare.com/cdn-cgi/trace";

    info!("Fetching with plain reqwest...");
    let start = tokio::time::Instant::now();
    let plain_response = reqwest::get(test_url).await?;
    let plain_status = plain_response.status();
    let plain_text = plain_response.text().await?;
    let plain_duration = start.elapsed();

    info!(
        "Plain reqwest - Status: {}, Time: {:?}",
        plain_status, plain_duration
    );

    info!("Setting up mixnet client...");
    let client = MixtcpReqwestClient::new().await?;
    let start = tokio::time::Instant::now();
    let mixnet_response = client.get(test_url).await?;
    let mixnet_status = mixnet_response.status();
    let mixnet_text = mixnet_response.text().await?;
    let mixnet_duration = start.elapsed();

    info!(
        "Mixnet reqwest - Status: {}, Time: {:?}",
        mixnet_status, mixnet_duration
    );

    info!("Status codes match: {}", plain_status == mixnet_status);
    info!(
        "Response lengths match: {}",
        plain_text.len() == mixnet_text.len()
    );

    let key_fields = ["fl=", "ip=", "ts=", "visit_scheme="];
    for field in key_fields {
        let plain_has = plain_text.contains(field);
        let mixnet_has = mixnet_text.contains(field);
        info!(
            "Field '{}' - Plain: {}, Mixnet: {}",
            field, plain_has, mixnet_has
        );
        assert_eq!(plain_has, mixnet_has, "Field '{}' mismatch", field);
    }

    info!("Plain reqwest time: {:?}", plain_duration);
    info!("Mixnet reqwest time: {:?}", mixnet_duration);
    let slowdown = mixnet_duration.as_millis() as f64 / plain_duration.as_millis() as f64;
    info!("Mixnet slowdown: {:.1}x", slowdown);
    info!("Both responses match");
    Ok(())
}
