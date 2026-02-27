#![allow(clippy::result_large_err)]
//! HTTPS request to Cloudflare's `/cdn-cgi/trace` endpoint through the mixnet.
//!
//! Performs a TCP+TLS handshake, sends an HTTP GET request, and reports split
//! timing for the handshake and request phases.
//!
//! Run with:
//!   cargo run --example cloudflare_ping

mod support;

use mixtcp::create_device;
use nym_sdk::stream_wrapper::{IpMixStream, NetworkEnvironment};
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp,
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
use std::io::Read;
use std::time::Duration;
use support::{BoxError, TlsOverTcp};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    support::init_logging();

    let ipr_stream = IpMixStream::new(NetworkEnvironment::Mainnet).await?;
    let (mut device, bridge, shutdown_handle, allocated_ips) = create_device(ipr_stream).await?;
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
    let mut success = false;
    let mut response_data = Vec::new();

    let mut handshake_duration = Duration::ZERO;
    let mut request_start = tokio::time::Instant::now();
    let mut request_duration = Duration::ZERO;

    loop {
        if start.elapsed() > Duration::from_secs(60) {
            info!("Test timeout after 60 seconds");
            break;
        }

        iface.poll(timestamp, &mut device, &mut sockets);
        timestamp += smoltcp::time::Duration::from_millis(1);
        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);

        // TCP connection setup
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

        // TLS setup after TCP established
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

        // TLS handshake and request
        if let Some(ref mut tls_conn) = tls {
            let _ = tls_conn.read_tls(socket);
            let _ = tls_conn.write_tls(socket);

            // Complete handshake
            if !tls_conn.conn.is_handshaking() && !handshake_completed {
                handshake_completed = true;
                handshake_duration = start.elapsed();
                info!("TCP+TLS handshake completed in {:?}", handshake_duration);

                // Send simple HTTP request
                request_start = tokio::time::Instant::now();
                let request = b"GET /cdn-cgi/trace HTTP/1.1\r\nHost: cloudflare.com\r\nUser-Agent: mixtcp-test/1.0\r\nAccept: */*\r\nConnection: close\r\n\r\n";
                match tls_conn.send(request, socket) {
                    Ok(_) => {
                        info!("HTTPS request sent");
                        request_sent = true;
                    }
                    Err(e) => {
                        info!("HTTPS send failed: {}", e);
                        break;
                    }
                }
            }

            // Read response after request sent
            if request_sent {
                let mut buf = vec![0u8; 4096];

                match tls_conn.conn.reader().read(&mut buf) {
                    Ok(0) => {
                        info!("Response complete - connection closed");
                        break;
                    }
                    Ok(n) => {
                        response_data.extend_from_slice(&buf[..n]);
                        info!("Received {} bytes", n);

                        if let Ok(response_str) = std::str::from_utf8(&response_data) {
                            if response_str.contains("\r\n\r\n") {
                                request_duration = request_start.elapsed();
                                info!("HTTPS response received!");

                                if let Some(status_end) = response_str.find("\r\n") {
                                    info!("HTTP Status: {}", &response_str[..status_end]);
                                }

                                info!("Full response: {}", response_str);
                                success = true;
                                break;
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // Keep polling
                    }
                    Err(e) => {
                        info!("Read error: {}", e);
                        break;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    info!("=== Timing ===");
    info!("TCP+TLS handshake: {:?}", handshake_duration);
    info!("HTTP request:      {:?}", request_duration);
    info!(
        "Total:             {:?}",
        handshake_duration + request_duration
    );

    shutdown_handle.shutdown();
    let _ = bridge_handle.await;

    if success {
        Ok(())
    } else {
        Err("No HTTP response received".into())
    }
}
