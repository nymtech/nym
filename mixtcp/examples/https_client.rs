#![allow(clippy::result_large_err)]
//! `reqwest`-like HTTPS `GET` client with timed clearnet comparison.
//!
//! Fetches the same URL over clearnet (via `reqwest`) and through the mixnet,
//! then compares response fields and reports timing.
//!
//! Run with:
//!   cargo run --example https_client

mod support;

use mixtcp::{create_device, NymIprDevice};
use nym_sdk::stream_wrapper::{IpMixStream, NetworkEnvironment};
use reqwest::StatusCode;
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp,
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;
use support::{BoxError, TlsOverTcp};
use tracing::info;

/// Reqwest-ish client right now, just a handrolled GET request for the example
pub struct MixtcpReqwestClient {
    device: Arc<tokio::sync::Mutex<(Interface, NymIprDevice)>>,
    bridge_handle: tokio::task::JoinHandle<()>,
    shutdown_handle: Option<mixtcp::BridgeShutdownHandle>,
    _allocated_ip: Ipv4Address,
}

impl MixtcpReqwestClient {
    pub async fn new() -> Result<Self, BoxError> {
        let ipr_stream = IpMixStream::new(NetworkEnvironment::Mainnet).await?;
        let (mut device, bridge, shutdown_handle, allocated_ips) =
            create_device(ipr_stream).await?;
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
            bridge_handle,
            shutdown_handle: Some(shutdown_handle),
            _allocated_ip: allocated_ips.ipv4,
        })
    }

    pub async fn shutdown(mut self) {
        if let Some(handle) = self.shutdown_handle.take() {
            handle.shutdown();
        }
        let _ = self.bridge_handle.await;
    }

    pub async fn get(&self, url: &str) -> Result<MixtcpResponse, BoxError> {
        let parsed_url = reqwest::Url::parse(url)?;
        let host = parsed_url.host_str().ok_or("URL has no host")?.to_string();
        let path = parsed_url.path().to_string();

        let (response_bytes, handshake_duration, request_duration) =
            self.simple_get_request(&host, &path).await?;
        let (status, body) = Self::parse_simple_response(&response_bytes)?;

        Ok(MixtcpResponse {
            status,
            body,
            handshake_duration,
            request_duration,
        })
    }

    async fn simple_get_request(
        &self,
        domain: &str,
        path: &str,
    ) -> Result<(Vec<u8>, Duration, Duration), BoxError> {
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

        let mut handshake_duration = Duration::ZERO;
        let mut request_start = tokio::time::Instant::now();

        let mut device_guard = self.device.lock().await;
        let (ref mut iface, ref mut device) = &mut *device_guard;

        loop {
            if start.elapsed() > Duration::from_secs(60) {
                return Err("request timeout".into());
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
                        return Err(format!("TCP connect failed: {e}").into());
                    }
                }
            }

            if socket.state() == tcp::State::Established && tls.is_none() {
                info!("TCP established - creating TLS connection");
                tls = Some(TlsOverTcp::new(domain)?);
            }

            if let Some(ref mut tls_conn) = tls {
                let _ = tls_conn.read_tls(socket);
                let _ = tls_conn.write_tls(socket);

                if !tls_conn.conn.is_handshaking() && !handshake_completed {
                    handshake_completed = true;
                    handshake_duration = start.elapsed();
                    info!(
                        "TCP+TLS handshake completed in {:?} - sending HTTP request",
                        handshake_duration
                    );

                    request_start = tokio::time::Instant::now();
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
                        Ok(n) => {
                            response_data.extend_from_slice(&buf[..n]);
                            if let Ok(response_str) = std::str::from_utf8(&response_data) {
                                if response_str.contains("\r\n\r\n") {
                                    let request_duration = request_start.elapsed();
                                    info!("HTTP response received in {:?}", request_duration);
                                    return Ok((
                                        response_data,
                                        handshake_duration,
                                        request_duration,
                                    ));
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(e) => {
                            return Err(format!("read error: {e}").into());
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Err("no response received".into())
    }

    /// Simple response - just extract status and body
    fn parse_simple_response(response_bytes: &[u8]) -> Result<(u16, String), BoxError> {
        let response_str = String::from_utf8_lossy(response_bytes);

        let status_line = response_str.lines().next().ok_or("empty HTTP response")?;

        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(200);

        if let Some(body_start) = response_str.find("\r\n\r\n") {
            let body = response_str[body_start + 4..].to_string();
            Ok((status, body))
        } else {
            Err("invalid HTTP response: no header/body separator".into())
        }
    }
}

pub struct MixtcpResponse {
    status: u16,
    body: String,
    handshake_duration: Duration,
    request_duration: Duration,
}

impl MixtcpResponse {
    pub fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub async fn text(self) -> Result<String, std::convert::Infallible> {
        Ok(self.body)
    }
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    support::init_logging();

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
    let mixnet_response = client.get(test_url).await?;
    let mixnet_status = mixnet_response.status();
    let handshake_duration = mixnet_response.handshake_duration;
    let request_duration = mixnet_response.request_duration;
    let mixnet_total = handshake_duration + request_duration;
    let mixnet_text = mixnet_response.text().await?;

    info!("Status codes match: {}", plain_status == mixnet_status);

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

    info!("=== Timing ===");
    info!("Clearnet (total):         {:?}", plain_duration);
    info!("Mixnet TCP+TLS handshake: {:?}", handshake_duration);
    info!("Mixnet HTTP request:      {:?}", request_duration);
    info!("Mixnet total:             {:?}", mixnet_total);
    let slowdown = mixnet_total.as_millis() as f64 / plain_duration.as_millis() as f64;
    info!("Mixnet slowdown:          {:.1}x", slowdown);

    client.shutdown().await;
    Ok(())
}
