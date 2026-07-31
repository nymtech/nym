// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the `smol-core` stack.
//!
//! Each test wires two stacks together with crossed channels (stack A's
//! outbound packets become stack B's inbound and vice versa), so the full path
//! — socket → smoltcp → ChannelDevice → transport → peer stack — is exercised
//! with no OS interface. This is the transport-agnostic seam in action.

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use futures::channel::mpsc;
use nym_smol_core::{ChannelDevice, DnsConfig, Stack, StackConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const A_IP: &str = "10.0.0.1";
const B_IP: &str = "10.0.0.2";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Build two stacks connected back-to-back through crossed IP-packet channels.
fn pair() -> (Stack, Stack) {
    let (a_out_tx, a_out_rx) = mpsc::unbounded::<Vec<u8>>();
    let (b_out_tx, b_out_rx) = mpsc::unbounded::<Vec<u8>>();

    // A's inbound = B's outbound; B's inbound = A's outbound.
    let a_dev = ChannelDevice::new(b_out_rx, a_out_tx, None);
    let b_dev = ChannelDevice::new(a_out_rx, b_out_tx, None);

    let a = Stack::new(a_dev, StackConfig::new(A_IP.parse().unwrap()));
    let b = Stack::new(b_dev, StackConfig::new(B_IP.parse().unwrap()));
    (a, b)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn udp_datagram_round_trip() {
    let (a, b) = pair();
    let server = b.udp_socket_on(9000).await.expect("bind server udp");
    let client = a.udp_socket().await.expect("bind client udp");

    client
        .send_to(b"ping", format!("{B_IP}:9000").parse().unwrap())
        .await
        .expect("send");

    let mut buf = [0u8; 64];
    let (n, src) = tokio::time::timeout(TIMEOUT, server.recv_from(&mut buf))
        .await
        .expect("server recv timed out")
        .expect("server recv");
    assert_eq!(&buf[..n], b"ping");

    server.send_to(b"pong", src).await.expect("reply");
    let (n, _) = tokio::time::timeout(TIMEOUT, client.recv_from(&mut buf))
        .await
        .expect("client recv timed out")
        .expect("client recv");
    assert_eq!(&buf[..n], b"pong");
}

// A multi-threaded runtime is required: each stack runs its own smoltcp reactor
// as a background task, and TCP's multi-round handshake needs them to progress
// concurrently with the test task.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tcp_stream_round_trip() {
    let (a, b) = pair();

    // Server side uses the underlying Net's TCP listener. It echoes the request,
    // then reads until EOF — staying open until the client has read the echo and
    // closed, so a premature drop can't RST the echo away.
    let mut listener = b
        .net()
        .tcp_bind("0.0.0.0:8080".parse().unwrap())
        .await
        .expect("tcp_bind");
    let server = tokio::spawn(async move {
        let (mut stream, _peer) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 5];
        stream.read_exact(&mut buf).await.expect("server read");
        stream.write_all(&buf).await.expect("server echo");
        stream.flush().await.expect("server flush");
        // Stay alive briefly so the echo is delivered before the socket drops
        // (avoids depending on FIN propagation through the loopback).
        tokio::time::sleep(Duration::from_secs(2)).await;
    });

    let mut client = tokio::time::timeout(
        TIMEOUT,
        a.tcp_connect(format!("{B_IP}:8080").parse().unwrap()),
    )
    .await
    .expect("connect timed out")
    .expect("tcp_connect");

    client.write_all(b"hello").await.expect("client write");
    let mut buf = [0u8; 5];
    tokio::time::timeout(TIMEOUT, client.read_exact(&mut buf))
        .await
        .expect("client read timed out")
        .expect("client read");
    assert_eq!(&buf, b"hello");

    // The server task self-terminates after its short grace period.
    let _ = tokio::time::timeout(TIMEOUT, server).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tcp_connect_to_closed_port_errors() {
    let (a, b) = pair();
    // b exists (so its smoltcp answers with RST) but nothing listens on 9999.
    let _ = &b;
    let result = tokio::time::timeout(
        TIMEOUT,
        a.tcp_connect(format!("{B_IP}:9999").parse().unwrap()),
    )
    .await;

    // Either the handshake was refused (Err) or it never completed (timeout).
    // The one thing that must NOT happen is a successful connection.
    if let Ok(Ok(_stream)) = result {
        panic!("connect unexpectedly succeeded to a closed port")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dns_resolves_over_stack_socket() {
    use hickory_proto::op::{Message, MessageType, OpCode};
    use hickory_proto::rr::{rdata::A, RData, Record, RecordType};

    let (a, b) = pair();
    let expected = Ipv4Addr::new(93, 184, 216, 34);

    // A mock DNS server bound on the peer stack: answers A queries, replies with
    // no records to anything else. Proves the query travels over a stack socket.
    let dns = b.udp_socket_on(53).await.expect("bind dns");
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        loop {
            let (n, src) = match dns.recv_from(&mut buf).await {
                Ok(x) => x,
                Err(_) => break,
            };
            let req = match Message::from_vec(&buf[..n]) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let query = req.queries[0].clone();
            let mut resp = Message::new(req.metadata.id, MessageType::Response, OpCode::Query);
            resp.add_query(query.clone());
            if query.query_type() == RecordType::A {
                resp.add_answer(Record::from_rdata(
                    query.name().clone(),
                    300,
                    RData::A(A(expected)),
                ));
            }
            let bytes = resp.to_vec().expect("encode dns response");
            let _ = dns.send_to(&bytes, src).await;
        }
    });

    let a = a.with_dns_config(DnsConfig {
        server: format!("{B_IP}:53").parse().unwrap(),
        timeout: TIMEOUT,
    });

    let addrs = tokio::time::timeout(TIMEOUT, a.resolve("example.com"))
        .await
        .expect("resolve timed out")
        .expect("resolve");
    assert!(
        addrs.contains(&IpAddr::V4(expected)),
        "resolved addrs {addrs:?} missing expected {expected}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_ip_literal_skips_dns() {
    // No DNS server is reachable on the peer; an IP-literal host must resolve immediately with no
    // query (a bogus lookup for "10.0.0.9" would hang/fail).
    let (a, _b) = pair();
    let addrs = tokio::time::timeout(Duration::from_secs(2), a.resolve("10.0.0.9"))
        .await
        .expect("IP-literal resolution must not block on DNS")
        .expect("resolve ip literal");
    assert_eq!(addrs, vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9))]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dns_ignores_mismatched_id() {
    use hickory_proto::op::{Message, MessageType, OpCode};
    use hickory_proto::rr::{rdata::A, RData, Record};

    let (a, b) = pair();
    let dns = b.udp_socket_on(53).await.expect("bind dns");
    // Reply with a deliberately wrong transaction id (a spoofing stand-in): the resolver must
    // discard it and, with no valid reply arriving, time out rather than accept the bogus address.
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        while let Ok((n, src)) = dns.recv_from(&mut buf).await {
            let req = match Message::from_vec(&buf[..n]) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let q = req.queries[0].clone();
            let mut resp = Message::new(
                req.metadata.id.wrapping_add(1),
                MessageType::Response,
                OpCode::Query,
            );
            resp.add_query(q.clone());
            resp.add_answer(Record::from_rdata(
                q.name().clone(),
                300,
                RData::A(A(Ipv4Addr::new(1, 2, 3, 4))),
            ));
            let _ = dns.send_to(&resp.to_vec().unwrap(), src).await;
        }
    });

    let a = a.with_dns_config(DnsConfig {
        server: format!("{B_IP}:53").parse().unwrap(),
        timeout: Duration::from_secs(2),
    });
    let err = tokio::time::timeout(TIMEOUT, a.resolve("example.com"))
        .await
        .expect("resolve should have returned")
        .expect_err("a mismatched-id response must not resolve");
    assert!(
        matches!(err, nym_smol_core::SmolCoreError::DnsTimeout { .. }),
        "expected DnsTimeout, got {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dns_servfail_is_distinct() {
    use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};

    let (a, b) = pair();
    let dns = b.udp_socket_on(53).await.expect("bind dns");
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        while let Ok((n, src)) = dns.recv_from(&mut buf).await {
            let req = match Message::from_vec(&buf[..n]) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let mut resp = Message::new(req.metadata.id, MessageType::Response, OpCode::Query);
            resp.add_query(req.queries[0].clone());
            resp.metadata.response_code = ResponseCode::ServFail;
            let _ = dns.send_to(&resp.to_vec().unwrap(), src).await;
        }
    });

    let a = a.with_dns_config(DnsConfig {
        server: format!("{B_IP}:53").parse().unwrap(),
        timeout: Duration::from_secs(3),
    });
    let err = tokio::time::timeout(TIMEOUT, a.resolve("example.com"))
        .await
        .expect("resolve should have returned")
        .expect_err("SERVFAIL must surface as an error");
    assert!(
        matches!(err, nym_smol_core::SmolCoreError::DnsServerFailure { .. }),
        "expected DnsServerFailure, got {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dns_v4_only_stack_skips_aaaa() {
    use hickory_proto::op::{Message, MessageType, OpCode};
    use hickory_proto::rr::{rdata::A, RData, Record, RecordType};
    use std::sync::{Arc, Mutex};

    let (a, b) = pair();
    let dns = b.udp_socket_on(53).await.expect("bind dns");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_srv = seen.clone();
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        while let Ok((n, src)) = dns.recv_from(&mut buf).await {
            let req = match Message::from_vec(&buf[..n]) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let q = req.queries[0].clone();
            seen_srv.lock().unwrap().push(q.query_type());
            let mut resp = Message::new(req.metadata.id, MessageType::Response, OpCode::Query);
            resp.add_query(q.clone());
            if q.query_type() == RecordType::A {
                resp.add_answer(Record::from_rdata(
                    q.name().clone(),
                    300,
                    RData::A(A(Ipv4Addr::new(5, 6, 7, 8))),
                ));
            }
            let _ = dns.send_to(&resp.to_vec().unwrap(), src).await;
        }
    });

    // Default StackConfig has no IPv6 address, so the resolver must query A only.
    let a = a.with_dns_config(DnsConfig {
        server: format!("{B_IP}:53").parse().unwrap(),
        timeout: TIMEOUT,
    });
    let addrs = tokio::time::timeout(TIMEOUT, a.resolve("example.com"))
        .await
        .expect("resolve timed out")
        .expect("resolve");
    assert!(addrs.contains(&IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8))));

    // Copy out and release the lock before asserting: a panic while holding it would poison the
    // mutex and make the still-running mock server panic on its next lock, burying this message.
    let seen = seen.lock().unwrap().clone();
    assert!(
        seen.iter().all(|t| *t == RecordType::A),
        "a v4-only stack must not send AAAA queries, saw {seen:?}"
    );
}

/// A stack whose inbound transport has gone away must not wedge runtime shutdown.
///
/// If the device reported end-of-stream to the smoltcp reactor, the reactor's `select!` would
/// complete instantly on every iteration - a loop that never returns `Poll::Pending`, so the tokio
/// worker running it can never be reclaimed and `Runtime::drop` blocks forever. Every test in this
/// file hit this on teardown: the first stack to drop closes its peer's inbound channel.
#[test]
fn dead_inbound_transport_does_not_wedge_runtime_shutdown() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build runtime");

    // Kept alive past the runtime drop below: the point is a live stack whose transport is dead.
    let _stack = rt.block_on(async {
        let (out_tx, _out_rx) = mpsc::unbounded::<Vec<u8>>();
        let (in_tx, in_rx) = mpsc::unbounded::<Vec<u8>>();
        let stack = Stack::new(
            ChannelDevice::new(in_rx, out_tx, None),
            StackConfig::new(A_IP.parse().unwrap()),
        );
        // Get the reactor running, then kill the inbound transport under it.
        let _sock = stack.udp_socket().await.expect("bind udp");
        drop(in_tx);
        tokio::time::sleep(Duration::from_millis(200)).await;
        (stack, _out_rx)
    });

    // Drop the runtime off-thread so a wedged shutdown fails the test instead of hanging it.
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        drop(rt);
        let _ = done_tx.send(());
    });
    done_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("runtime shutdown wedged by a stack whose inbound transport was dropped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dns_rejects_truncated_response() {
    use hickory_proto::op::{Message, MessageType, OpCode};
    use hickory_proto::rr::{rdata::A, RData, Record};

    let (a, b) = pair();
    let dns = b.udp_socket_on(53).await.expect("bind dns");
    // Reply with the TC (truncation) bit set — and include an A answer, to prove the resolver
    // rejects a truncated response rather than returning the partial answer it carries.
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        while let Ok((n, src)) = dns.recv_from(&mut buf).await {
            let req = match Message::from_vec(&buf[..n]) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let q = req.queries[0].clone();
            let mut resp = Message::new(req.metadata.id, MessageType::Response, OpCode::Query);
            resp.metadata.truncation = true;
            resp.add_query(q.clone());
            resp.add_answer(Record::from_rdata(
                q.name().clone(),
                300,
                RData::A(A(Ipv4Addr::new(9, 9, 9, 9))),
            ));
            let _ = dns.send_to(&resp.to_vec().unwrap(), src).await;
        }
    });

    let a = a.with_dns_config(DnsConfig {
        server: format!("{B_IP}:53").parse().unwrap(),
        timeout: Duration::from_secs(3),
    });
    let err = tokio::time::timeout(TIMEOUT, a.resolve("example.com"))
        .await
        .expect("resolve should have returned")
        .expect_err("a truncated response must not resolve");
    assert!(
        matches!(err, nym_smol_core::SmolCoreError::DnsTruncated { .. }),
        "expected DnsTruncated, got {err:?}"
    );
}
