#![allow(clippy::result_large_err)]
//! TLS handshake diagnostics through the mixnet.
//!
//! Connects to Cloudflare (1.1.1.1:443) through a smoltcp TCP socket tunneled
//! over the mixnet, performs a TLS handshake, and logs connection state at
//! regular intervals.
//!
//! Run with:
//!   cargo run --example tls

mod support;

use mixtcp::create_device;
use nym_sdk::stream_wrapper::{IpMixStream, NetworkEnvironment};
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp,
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
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

    let target_ip = Ipv4Address::new(1, 1, 1, 1); // Pinging Cloudflare
    let target_port = 443;
    info!("Connecting to {}:{} through mixnet", target_ip, target_port);

    let mut timestamp = Instant::from_millis(0);
    let start = tokio::time::Instant::now();
    let mut connected = false;
    let mut tls = None;
    let mut handshake_completed = false;
    let mut tcp_established_duration = Duration::ZERO;
    let mut tls_handshake_start = tokio::time::Instant::now();

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

        if start.elapsed().as_secs().is_multiple_of(5) && start.elapsed().as_millis() % 1000 < 100 {
            info!(
                "State: TCP={:?}, established={}, can_send={}, can_recv={}",
                socket.state(),
                socket.state() == tcp::State::Established,
                socket.may_send(),
                socket.can_recv()
            );
        }

        if socket.state() == tcp::State::Established && tls.is_none() {
            tcp_established_duration = start.elapsed();
            info!(
                "TCP established in {:?} - creating TLS connection",
                tcp_established_duration
            );
            tls_handshake_start = tokio::time::Instant::now();
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

            if start.elapsed().as_secs().is_multiple_of(10)
                && start.elapsed().as_millis() % 1000 < 100
            {
                info!(
                    "TLS state: handshaking={}, wants_read={}, wants_write={}",
                    tls_conn.conn.is_handshaking(),
                    tls_conn.conn.wants_read(),
                    tls_conn.conn.wants_write()
                );
            }

            if !tls_conn.conn.is_handshaking() && !handshake_completed {
                #[allow(unused_assignments)]
                {
                    handshake_completed = true;
                }
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
                let tls_handshake_duration = tls_handshake_start.elapsed();
                let total = start.elapsed();
                info!("TLS handshake successful with cloudflare");
                info!("=== Timing ===");
                info!("TCP established:  {:?}", tcp_established_duration);
                info!("TLS handshake:    {:?}", tls_handshake_duration);
                info!("Total:            {:?}", total);
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    shutdown_handle.shutdown();
    let _ = bridge_handle.await;
    info!("Test completed");
    Ok(())
}
