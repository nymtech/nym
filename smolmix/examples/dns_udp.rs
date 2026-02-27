#![allow(clippy::result_large_err)]
//! UDP example: DNS A-record query through the Nym mixnet.
//!
//! Sends a query for `example.com` to Cloudflare DNS (1.1.1.1:53) over a
//! smoltcp UDP socket tunneled through the mixnet, then compares the result
//! against a standard clearnet DNS lookup using hickory-resolver.
//!
//! Run with:
//!   cargo run --example dns_udp

use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;

use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use hickory_resolver::TokioResolver;
use nym_sdk::stream_wrapper::{IpMixStream, NetworkEnvironment};
use smolmix::create_device;
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::udp,
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address},
};
use tracing::info;

/// Build a DNS A-record query using hickory-proto.
fn build_query(domain: &str, id: u16) -> Vec<u8> {
    let mut msg = Message::new();
    msg.set_id(id);
    msg.set_message_type(MessageType::Query);
    msg.set_op_code(OpCode::Query);
    msg.set_recursion_desired(true);

    let name = Name::from_str(domain).expect("invalid domain");
    msg.add_query(Query::query(name, RecordType::A));

    msg.to_vec().expect("failed to serialize DNS query")
}

/// Extract A-record IPv4 addresses from a DNS response.
fn parse_response(bytes: &[u8]) -> Vec<Ipv4Addr> {
    let msg = match Message::from_vec(bytes) {
        Ok(m) => m,
        Err(e) => {
            info!("Failed to parse DNS response: {}", e);
            return vec![];
        }
    };

    info!(
        "DNS response: id={:#06x}, rcode={}, answers={}",
        msg.id(),
        msg.response_code(),
        msg.answers().len()
    );

    msg.answers()
        .iter()
        .filter_map(|record| match record.data() {
            RData::A(a) => Some(a.0),
            _ => None,
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    nym_bin_common::logging::setup_tracing_logger();

    let domain = "example.com";
    let dns_server = Ipv4Address::new(1, 1, 1, 1);

    // --- Clearnet baseline using hickory-resolver ---
    info!("Clearnet DNS lookup for '{}'...", domain);
    let resolver = TokioResolver::builder_tokio()?.build();
    let clearnet_start = tokio::time::Instant::now();
    let lookup = resolver.lookup_ip(domain).await?;
    let clearnet_ips: Vec<Ipv4Addr> = lookup
        .iter()
        .filter_map(|ip| match ip {
            std::net::IpAddr::V4(v4) => Some(v4),
            _ => None,
        })
        .collect();
    let clearnet_duration = clearnet_start.elapsed();
    info!(
        "Clearnet resolved: {:?} in {:?}",
        clearnet_ips, clearnet_duration
    );

    // --- Mixnet UDP lookup ---
    info!("Connecting to mixnet...");
    let ipr_stream = IpMixStream::new(NetworkEnvironment::Mainnet).await?;
    let (mut device, bridge, shutdown_handle, ips) = create_device(ipr_stream).await?;
    info!("Allocated IP: {}", ips.ipv4);

    let bridge_handle = tokio::spawn(async move {
        if let Err(e) = bridge.run().await {
            tracing::error!("Bridge error: {}", e);
        }
    });

    // Configure interface
    let config = Config::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, &mut device, Instant::now());
    iface.update_ip_addrs(|addrs| {
        addrs
            .push(IpCidr::new(IpAddress::from(ips.ipv4), 32))
            .unwrap();
    });
    iface
        .routes_mut()
        .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
        .unwrap();

    // Create UDP socket
    let rx_buf = udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 4], vec![0; 2048]);
    let tx_buf = udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 4], vec![0; 2048]);
    let mut sockets = SocketSet::new(vec![]);
    let handle = sockets.add(udp::Socket::new(rx_buf, tx_buf));
    sockets.get_mut::<udp::Socket>(handle).bind(10053)?;

    // Build query and send
    let query_bytes = build_query(domain, 0xABCD);
    let dest = IpEndpoint::new(IpAddress::from(dns_server), 53);
    let mut timestamp = Instant::from_millis(0);
    let mixnet_start = tokio::time::Instant::now();
    let mut sent = false;

    info!("Sending DNS query via mixnet UDP...");

    let mut success = false;

    loop {
        if mixnet_start.elapsed() > Duration::from_secs(120) {
            break;
        }

        iface.poll(timestamp, &mut device, &mut sockets);
        timestamp += smoltcp::time::Duration::from_millis(1);

        let sock = sockets.get_mut::<udp::Socket>(handle);

        if !sent && sock.can_send() {
            sock.send_slice(&query_bytes, dest)?;
            info!(
                "Sent {} byte DNS query to {}:53",
                query_bytes.len(),
                dns_server
            );
            sent = true;
        }

        if sent && sock.can_recv() {
            match sock.recv() {
                Ok((data, endpoint)) => {
                    let mixnet_duration = mixnet_start.elapsed();
                    info!(
                        "Got {} byte response from {} in {:?}",
                        data.len(),
                        endpoint,
                        mixnet_duration
                    );

                    let mixnet_ips = parse_response(data);

                    // --- Compare results ---
                    info!("=== Results ===");
                    info!("Clearnet: {:?} ({:?})", clearnet_ips, clearnet_duration);
                    info!("Mixnet:   {:?} ({:?})", mixnet_ips, mixnet_duration);

                    let match_ok = !mixnet_ips.is_empty()
                        && mixnet_ips.iter().all(|ip| clearnet_ips.contains(ip));
                    info!("IPs match: {}", match_ok);

                    let slowdown =
                        mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis() as f64;
                    info!("Mixnet slowdown: {:.1}x", slowdown);

                    success = true;
                    break;
                }
                Err(udp::RecvError::Truncated) => info!("Truncated packet, continuing"),
                Err(udp::RecvError::Exhausted) => {}
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    shutdown_handle.shutdown();
    let _ = bridge_handle.await;

    if success {
        Ok(())
    } else {
        Err("Timed out waiting for DNS response".into())
    }
}
