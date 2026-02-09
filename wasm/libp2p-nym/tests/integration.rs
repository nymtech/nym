// Copyright 2024 Nym Technologies SA
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the Nym libp2p WASM transport.
//!
//! These tests run in a browser environment via wasm-bindgen-test.
//! Run with: `wasm-pack test --headless --chrome`
//!
//! Note: These tests require network connectivity to the Mixnet.
//! Each test creates real Nym clients that connect to gateways.
//!
//! Tests run sequentially (wasm-bindgen-test default in browser).

#![cfg(target_arch = "wasm32")]

use futures::{select, Future, FutureExt, StreamExt};
use libp2p::core::transport::{DialOpts, PortUse, TransportEvent};
use libp2p::core::Endpoint;
use libp2p::core::{Multiaddr, Transport};
use libp2p_identity::Keypair;
use std::pin::Pin;
use std::str::FromStr;
use std::task::Poll;
use wasm_bindgen_test::*;

use nym_libp2p_wasm::{
    create_transport_client_async, nym_address_to_multiaddress, NymTransport, TransportClientOpts,
};

wasm_bindgen_test_configure!(run_in_browser);

// Helper Functions

/// Log to browser console.
macro_rules! console_log {
    ($($t:tt)*) => {
        web_sys::console::log_1(&format!($($t)*).into())
    };
}

/// Create DialOpts
fn dial_opts() -> DialOpts {
    DialOpts {
        role: Endpoint::Dialer,
        port_use: PortUse::New,
    }
}

/// Create a transport client with unique ID to avoid storage conflicts.
/// Returns the transport and the listen multiaddr.
async fn create_test_transport(name: &str) -> (NymTransport, Multiaddr) {
    let opts = TransportClientOpts {
        nym_api_url: None,
        force_tls: true,
        client_id: Some(format!("test-{}-{}", name, js_sys::Date::now() as u64)),
    };

    console_log!("Creating transport client for '{}'...", name);

    let result = create_transport_client_async(opts)
        .await
        .expect("Failed to create transport client");

    let address_str = result.self_address.to_string();
    console_log!("Got address for '{}': {}", name, &address_str[..20]);

    let multiaddr =
        nym_address_to_multiaddress(result.self_address).expect("Failed to create multiaddr");

    let keypair = Keypair::generate_ed25519();

    let transport = NymTransport::new(result.self_address, result.stream, keypair)
        .await
        .expect("Failed to create transport");

    console_log!("Transport '{}' created successfully", name);

    (transport, multiaddr)
}

/// Simple async sleep using gloo_timers.
async fn sleep_ms(ms: u32) {
    gloo_timers::future::TimeoutFuture::new(ms).await;
}

// Test 1: Basic Transport Creation

#[wasm_bindgen_test]
async fn test_01_transport_creation() {
    console_log!("=== test_01_transport_creation ===");

    let (transport, multiaddr) = create_test_transport("creation").await;

    let addr_str = multiaddr.to_string();

    // Address should be in /nym/<base58> format
    assert!(
        addr_str.starts_with("/nym/"),
        "Should be Nym multiaddr: {}",
        addr_str
    );
    assert!(addr_str.len() > 60, "Address too short: {}", addr_str);

    // Nym addresses contain '@' and '.' in the base58 part
    assert!(
        addr_str.contains('@') && addr_str.contains('.'),
        "Address should be in Nym recipient format: {}",
        addr_str
    );

    console_log!("test_01_transport_creation PASSED");

    // Cleanup: drop transport to shutdown Nym client
    drop(transport);
    console_log!("Transport dropped, client shutting down...");
    sleep_ms(500).await; // Give time for cleanup
}

// Test 2: Unique Addresses

#[wasm_bindgen_test]
async fn test_02_unique_addresses() {
    console_log!("=== test_02_unique_addresses ===");

    let (transport_a, addr_a) = create_test_transport("unique-a").await;
    let (transport_b, addr_b) = create_test_transport("unique-b").await;

    assert_ne!(addr_a, addr_b, "Transports should have unique addresses");

    console_log!("test_02_unique_addresses PASSED");

    // Cleanup
    drop(transport_a);
    drop(transport_b);
    console_log!("Transports dropped...");
    sleep_ms(500).await;
}

// Test 3: Invalid Address Handling

#[wasm_bindgen_test]
async fn test_03_dial_invalid_address() {
    console_log!("=== test_03_dial_invalid_address ===");

    let (mut transport, _addr) = create_test_transport("invalid").await;

    // Try to dial a malformed address (not a valid Nym recipient)
    let bad_addr = Multiaddr::from_str("/nym/not-a-valid-nym-address").unwrap();

    let result = transport.dial(bad_addr, dial_opts());

    // Should fail at dial time (parsing the address)
    assert!(result.is_err(), "Dialing invalid address should fail");

    console_log!("test_03_dial_invalid_address PASSED");

    // Cleanup
    drop(transport);
    sleep_ms(500).await;
}

// Test 4: Dial and Accept Connection

#[wasm_bindgen_test]
async fn test_04_dial_and_accept() {
    console_log!("=== test_04_dial_and_accept ===");

    // Create two transports
    let (mut transport_a, addr_a) = create_test_transport("dial-a").await;
    let (mut transport_b, addr_b) = create_test_transport("dial-b").await;

    console_log!("Transport A: {}", addr_a);
    console_log!("Transport B: {}", addr_b);
    console_log!("A dialing B...");

    // A dials B
    let dial_future = transport_a
        .dial(addr_b.clone(), dial_opts())
        .expect("dial() should not fail immediately");

    let mut dial_future = Box::pin(dial_future);
    let mut got_incoming = false;
    let mut dial_complete = false;
    let mut _conn_a = None;
    let mut _conn_b = None;

    // Poll loop with timeout
    let start = js_sys::Date::now();
    let timeout_ms = 90_000.0;

    while (!got_incoming || !dial_complete) && (js_sys::Date::now() - start) < timeout_ms {
        // Poll transport B for incoming connection
        if !got_incoming {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);

            match Transport::poll(Pin::new(&mut transport_b), &mut cx) {
                Poll::Ready(event) => match event {
                    TransportEvent::Incoming {
                        upgrade,
                        local_addr,
                        send_back_addr,
                        ..
                    } => {
                        console_log!(
                            "B: Incoming connection! local={}, remote={}",
                            local_addr,
                            send_back_addr
                        );
                        match upgrade.await {
                            Ok((peer_id, conn)) => {
                                console_log!("B: Connection accepted from {:?}", peer_id);
                                _conn_b = Some(conn);
                                got_incoming = true;
                            }
                            Err(e) => console_log!("B: Upgrade failed: {:?}", e),
                        }
                    }
                    TransportEvent::NewAddress { listen_addr, .. } => {
                        console_log!("B: NewAddress: {}", listen_addr);
                    }
                    other => console_log!("B: Event {:?}", other),
                },
                Poll::Pending => {}
            }
        }

        // Poll the dial future
        if !dial_complete {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);

            match dial_future.as_mut().poll(&mut cx) {
                Poll::Ready(Ok((peer_id, conn))) => {
                    console_log!("A: Dial complete! Connected to {:?}", peer_id);
                    _conn_a = Some(conn);
                    dial_complete = true;
                }
                Poll::Ready(Err(e)) => panic!("A: Dial failed: {:?}", e),
                Poll::Pending => {}
            }
        }

        // Poll transport A to process inbound messages
        {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);
            let _ = Transport::poll(Pin::new(&mut transport_a), &mut cx);
        }

        sleep_ms(100).await;
    }

    let elapsed = (js_sys::Date::now() - start) / 1000.0;
    console_log!(
        "Poll loop finished after {:.1}s - incoming={}, dial={}",
        elapsed,
        got_incoming,
        dial_complete
    );

    assert!(
        got_incoming,
        "Transport B should receive incoming connection"
    );
    assert!(dial_complete, "Transport A dial should complete");

    console_log!("test_04_dial_and_accept PASSED");

    // Cleanup: drop connections first, then transports
    drop(_conn_a);
    drop(_conn_b);
    drop(transport_a);
    drop(transport_b);
    console_log!("All resources dropped...");
    sleep_ms(1000).await; // Longer cleanup for connection resources
}

// Test 5: Substream Data Exchange with SURB Anonymous Replies

#[wasm_bindgen_test]
async fn test_05_substream_data_exchange() {
    use futures::{AsyncRead, AsyncWriteExt};
    use libp2p::core::StreamMuxer;

    console_log!("=== test_05_substream_data_exchange ===");

    // Create two transports
    let (mut transport_a, addr_a) = create_test_transport("stream-a").await;
    let (mut transport_b, addr_b) = create_test_transport("stream-b").await;

    console_log!("Transport A: {}", addr_a);
    console_log!("Transport B: {}", addr_b);

    // A dials B
    let dial_future = transport_a
        .dial(addr_b, dial_opts())
        .expect("dial() should not fail");

    let mut dial_future = Box::pin(dial_future);
    let mut conn_a: Option<nym_libp2p_wasm::Connection> = None;
    let mut conn_b: Option<nym_libp2p_wasm::Connection> = None;

    let start = js_sys::Date::now();
    let timeout_ms = 90_000.0;

    console_log!("Establishing connection...");

    // First, establish the connection
    while (conn_a.is_none() || conn_b.is_none()) && (js_sys::Date::now() - start) < timeout_ms {
        // Poll B for incoming
        if conn_b.is_none() {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);

            match Transport::poll(Pin::new(&mut transport_b), &mut cx) {
                Poll::Ready(TransportEvent::Incoming { upgrade, .. }) => match upgrade.await {
                    Ok((peer_id, conn)) => {
                        console_log!("B: Connection from {:?}", peer_id);
                        conn_b = Some(conn);
                    }
                    Err(e) => console_log!("B: Upgrade failed: {:?}", e),
                },
                Poll::Ready(TransportEvent::NewAddress { .. }) => {}
                Poll::Ready(other) => console_log!("B: Event {:?}", other),
                Poll::Pending => {}
            }
        }

        // Poll dial future
        if conn_a.is_none() {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);

            match dial_future.as_mut().poll(&mut cx) {
                Poll::Ready(Ok((peer_id, conn))) => {
                    console_log!("A: Connected to {:?}", peer_id);
                    conn_a = Some(conn);
                }
                Poll::Ready(Err(e)) => panic!("A: Dial failed: {:?}", e),
                Poll::Pending => {}
            }
        }

        // Poll transport A for message routing
        {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);
            let _ = Transport::poll(Pin::new(&mut transport_a), &mut cx);
        }

        sleep_ms(100).await;
    }

    let mut conn_a = conn_a.expect("A should have connection");
    let mut conn_b = conn_b.expect("B should have connection");

    console_log!("Connection established! Opening substream...");

    // A opens an outbound substream
    let waker = futures::task::noop_waker();
    let mut cx = std::task::Context::from_waker(&waker);

    let mut substream_a = match StreamMuxer::poll_outbound(Pin::new(&mut conn_a), &mut cx) {
        Poll::Ready(Ok(s)) => {
            console_log!("A: Opened outbound substream");
            s
        }
        Poll::Ready(Err(e)) => panic!("A: Failed to open substream: {:?}", e),
        Poll::Pending => panic!("A: poll_outbound returned Pending"),
    };

    // Wait for B to receive the OpenRequest and create inbound substream
    let mut substream_b: Option<nym_libp2p_wasm::Substream> = None;
    let start = js_sys::Date::now();

    console_log!("Waiting for B to receive substream open request...");

    while substream_b.is_none() && (js_sys::Date::now() - start) < timeout_ms {
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        // Drain all pending transport messages
        loop {
            let poll_a = Transport::poll(Pin::new(&mut transport_a), &mut cx);
            let poll_b = Transport::poll(Pin::new(&mut transport_b), &mut cx);
            if poll_a.is_pending() && poll_b.is_pending() {
                break;
            }
        }

        // Drain all pending connection messages
        while StreamMuxer::poll(Pin::new(&mut conn_b), &mut cx).is_ready() {}

        // Check for inbound substream
        match StreamMuxer::poll_inbound(Pin::new(&mut conn_b), &mut cx) {
            Poll::Ready(Ok(s)) => {
                console_log!("B: Received inbound substream");
                substream_b = Some(s);
            }
            Poll::Ready(Err(e)) => console_log!("B: poll_inbound error: {:?}", e),
            Poll::Pending => {}
        }

        sleep_ms(100).await;
    }

    let mut substream_b = substream_b.expect("B should receive inbound substream");

    console_log!("Substream established! Testing data exchange...");

    // === Test 1: A sends to B ===
    let msg_a_to_b = b"Hello B! This message travels through the mixnet.";

    substream_a
        .write_all(msg_a_to_b)
        .await
        .expect("A: write should succeed");
    console_log!("A: Sent {} bytes to B", msg_a_to_b.len());

    // B reads the message
    let mut buf = vec![0u8; 1024];
    let mut total_read = 0;
    let start = js_sys::Date::now();

    while total_read < msg_a_to_b.len() && (js_sys::Date::now() - start) < timeout_ms {
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        // Drain all pending transport messages
        loop {
            let poll_a = Transport::poll(Pin::new(&mut transport_a), &mut cx);
            let poll_b = Transport::poll(Pin::new(&mut transport_b), &mut cx);
            if poll_a.is_pending() && poll_b.is_pending() {
                break;
            }
        }

        // Drain all pending connection messages
        while StreamMuxer::poll(Pin::new(&mut conn_b), &mut cx).is_ready() {}

        // Try to read
        match Pin::new(&mut substream_b).poll_read(&mut cx, &mut buf[total_read..]) {
            Poll::Ready(Ok(n)) if n > 0 => {
                total_read += n;
                console_log!("B: Read {} bytes (total: {})", n, total_read);
            }
            Poll::Ready(Ok(_)) => {}
            Poll::Ready(Err(e)) => panic!("B: Read error: {:?}", e),
            Poll::Pending => {}
        }

        sleep_ms(100).await;
    }

    assert_eq!(total_read, msg_a_to_b.len(), "B should receive all bytes");
    assert_eq!(
        &buf[..total_read],
        msg_a_to_b,
        "B should receive correct data"
    );
    console_log!(
        "B: Received message: {:?}",
        String::from_utf8_lossy(&buf[..total_read])
    );

    // === Test 2: B replies to A using SURBs (anonymous reply!) ===
    // B never learns A's Nym address - it replies using the SURB A provided
    let msg_b_to_a = b"Hello A! I'm replying anonymously via SURB - I don't know your address!";

    substream_b
        .write_all(msg_b_to_a)
        .await
        .expect("B: write should succeed");
    console_log!(
        "B: Sent anonymous reply ({} bytes) via SURB",
        msg_b_to_a.len()
    );

    // A reads the reply
    let mut buf_a = vec![0u8; 1024];
    let mut total_read_a = 0;
    let start = js_sys::Date::now();

    while total_read_a < msg_b_to_a.len() && (js_sys::Date::now() - start) < timeout_ms {
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        // Drain all pending transport messages
        loop {
            let poll_a = Transport::poll(Pin::new(&mut transport_a), &mut cx);
            let poll_b = Transport::poll(Pin::new(&mut transport_b), &mut cx);
            if poll_a.is_pending() && poll_b.is_pending() {
                break;
            }
        }

        // Drain all pending connection messages
        while StreamMuxer::poll(Pin::new(&mut conn_a), &mut cx).is_ready() {}

        // Try to read
        match Pin::new(&mut substream_a).poll_read(&mut cx, &mut buf_a[total_read_a..]) {
            Poll::Ready(Ok(n)) if n > 0 => {
                total_read_a += n;
                console_log!("A: Read {} bytes (total: {})", n, total_read_a);
            }
            Poll::Ready(Ok(_)) => {}
            Poll::Ready(Err(e)) => panic!("A: Read error: {:?}", e),
            Poll::Pending => {}
        }

        sleep_ms(100).await;
    }

    assert_eq!(total_read_a, msg_b_to_a.len(), "A should receive all bytes");
    assert_eq!(
        &buf_a[..total_read_a],
        msg_b_to_a,
        "A should receive correct data"
    );
    console_log!(
        "A: Received reply: {:?}",
        String::from_utf8_lossy(&buf_a[..total_read_a])
    );

    console_log!("=== test_05_substream_data_exchange PASSED ===");
    console_log!("Successfully demonstrated:");
    console_log!("  - A -> B: Direct message through mixnet");
    console_log!("  - B -> A: Anonymous reply via SURB (B never learned A's address!)");

    // Cleanup: drop in order - substreams, connections, transports
    drop(substream_a);
    drop(substream_b);
    drop(conn_a);
    drop(conn_b);
    drop(transport_a);
    drop(transport_b);
    console_log!("All resources dropped...");
    sleep_ms(1000).await;
}

// Test 6: Privacy Validation - Verify Sender Anonymity

/// Verify that the receiver (B) cannot learn the sender's (A) Nym address.
///
/// This test validates the core privacy property of the transport:
/// - B can receive messages from A
/// - B can reply to A via SURBs
/// - B CANNOT see A's Nym address
/// - B sees a fresh PeerId each time (unlinkability)
#[wasm_bindgen_test]
async fn test_06_privacy_sender_anonymity() {
    console_log!("=== test_06_privacy_sender_anonymity ===");

    // Create two transports
    let (mut transport_a, addr_a) = create_test_transport("privacy-a").await;
    let (mut transport_b, _addr_b) = create_test_transport("privacy-b").await;

    // Store A's address string to verify B doesn't see it
    let a_address_str = addr_a.to_string();
    console_log!("A's Nym address (B should NOT see this): {}", a_address_str);

    // A dials B
    let dial_future = transport_a
        .dial(_addr_b, dial_opts())
        .expect("dial() should not fail");

    let mut dial_future = Box::pin(dial_future);
    let mut conn_a: Option<nym_libp2p_wasm::Connection> = None;
    let mut conn_b: Option<nym_libp2p_wasm::Connection> = None;

    let start = js_sys::Date::now();
    let timeout_ms = 90_000.0;

    // Establish connection
    while (conn_a.is_none() || conn_b.is_none()) && (js_sys::Date::now() - start) < timeout_ms {
        if conn_b.is_none() {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);

            match Transport::poll(Pin::new(&mut transport_b), &mut cx) {
                Poll::Ready(TransportEvent::Incoming { upgrade, .. }) => match upgrade.await {
                    Ok((peer_id, conn)) => {
                        console_log!("B: Connection from PeerId {:?}", peer_id);
                        conn_b = Some(conn);
                    }
                    Err(e) => console_log!("B: Upgrade failed: {:?}", e),
                },
                Poll::Ready(TransportEvent::NewAddress { .. }) => {}
                Poll::Ready(other) => console_log!("B: Event {:?}", other),
                Poll::Pending => {}
            }
        }

        if conn_a.is_none() {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);

            match dial_future.as_mut().poll(&mut cx) {
                Poll::Ready(Ok((peer_id, conn))) => {
                    console_log!("A: Connected to {:?}", peer_id);
                    conn_a = Some(conn);
                }
                Poll::Ready(Err(e)) => panic!("A: Dial failed: {:?}", e),
                Poll::Pending => {}
            }
        }

        {
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);
            let _ = Transport::poll(Pin::new(&mut transport_a), &mut cx);
        }

        sleep_ms(100).await;
    }

    let conn_a = conn_a.expect("A should have connection");
    let conn_b = conn_b.expect("B should have connection");

    // === PRIVACY VALIDATION ===

    console_log!("=== Privacy Validation ===");

    // 1. Verify B does NOT know A's Nym address
    let b_sees_remote_address = conn_b.remote_nym_address();
    assert!(
        b_sees_remote_address.is_none(),
        "PRIVACY VIOLATION: B should NOT be able to see A's Nym address!"
    );
    console_log!("✓ B cannot see A's Nym address (remote_nym_address = None)");

    // 2. Verify B uses anonymous replies (SURBs)
    assert!(
        conn_b.uses_anonymous_replies(),
        "B should be using anonymous replies (SURBs)"
    );
    console_log!("✓ B uses anonymous replies via SURBs");

    // 3. Verify A knows B's address (A initiated the connection)
    let a_sees_remote_address = conn_a.remote_nym_address();
    assert!(
        a_sees_remote_address.is_some(),
        "A should know B's address (A dialed B)"
    );
    console_log!("✓ A knows B's Nym address (as expected - A initiated)");

    // 4. Verify A does NOT use SURBs (A sends directly to B's address)
    assert!(
        !conn_a.uses_anonymous_replies(),
        "A should NOT be using SURBs (A knows B's address)"
    );
    console_log!("✓ A sends directly to B's address (no SURBs needed)");

    // 5. Log what B CAN see (for documentation)
    let peer_id_b_sees = conn_b.peer_id();
    console_log!("What B CAN see:");
    console_log!(
        "  - PeerId: {:?} (fresh per connection, unlinkable)",
        peer_id_b_sees
    );
    console_log!("  - Message content (encrypted at higher layers in production)");
    console_log!("What B CANNOT see:");
    console_log!("  - A's Nym address: HIDDEN");
    console_log!("  - A's gateway: HIDDEN");
    console_log!("  - A's IP address: HIDDEN");

    console_log!("=== test_06_privacy_sender_anonymity PASSED ===");

    // Cleanup
    drop(conn_a);
    drop(conn_b);
    drop(transport_a);
    drop(transport_b);
    sleep_ms(1000).await;
}

// Test 7: PeerId Unlinkability - Fresh PeerId Per Connection

/// Verify that each outgoing connection gets a fresh PeerId.
///
/// This ensures that if A connects to B multiple times, B cannot link
/// the connections as coming from the same peer.
#[wasm_bindgen_test]
async fn test_07_peerid_unlinkability() {
    console_log!("=== test_07_peerid_unlinkability ===");

    // Create transports
    let (mut transport_a, _addr_a) = create_test_transport("unlink-a").await;
    let (mut transport_b, addr_b) = create_test_transport("unlink-b").await;

    // First connection: A dials B
    console_log!("First connection attempt...");
    let dial1 = transport_a
        .dial(addr_b.clone(), dial_opts())
        .expect("dial should work");

    let mut dial1 = Box::pin(dial1);
    let mut peer_id_1: Option<libp2p_identity::PeerId> = None;

    let start = js_sys::Date::now();
    let timeout_ms = 90_000.0;

    while peer_id_1.is_none() && (js_sys::Date::now() - start) < timeout_ms {
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        // Poll B
        match Transport::poll(Pin::new(&mut transport_b), &mut cx) {
            Poll::Ready(TransportEvent::Incoming { upgrade, .. }) => {
                if let Ok((peer_id, conn)) = upgrade.await {
                    console_log!("Connection 1: B sees PeerId {:?}", peer_id);
                    peer_id_1 = Some(peer_id);
                    drop(conn);
                }
            }
            _ => {}
        }

        // Poll dial
        if let Poll::Ready(Ok((_, conn))) = dial1.as_mut().poll(&mut cx) {
            drop(conn);
        }

        let _ = Transport::poll(Pin::new(&mut transport_a), &mut cx);
        sleep_ms(100).await;
    }

    let peer_id_1 = peer_id_1.expect("Should get first connection");
    sleep_ms(500).await;

    // Second connection: A dials B again
    console_log!("Second connection attempt...");
    let dial2 = transport_a
        .dial(addr_b.clone(), dial_opts())
        .expect("dial should work");

    let mut dial2 = Box::pin(dial2);
    let mut peer_id_2: Option<libp2p_identity::PeerId> = None;

    let start = js_sys::Date::now();

    while peer_id_2.is_none() && (js_sys::Date::now() - start) < timeout_ms {
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        // Poll B
        match Transport::poll(Pin::new(&mut transport_b), &mut cx) {
            Poll::Ready(TransportEvent::Incoming { upgrade, .. }) => {
                if let Ok((peer_id, conn)) = upgrade.await {
                    console_log!("Connection 2: B sees PeerId {:?}", peer_id);
                    peer_id_2 = Some(peer_id);
                    drop(conn);
                }
            }
            _ => {}
        }

        // Poll dial
        if let Poll::Ready(Ok((_, conn))) = dial2.as_mut().poll(&mut cx) {
            drop(conn);
        }

        let _ = Transport::poll(Pin::new(&mut transport_a), &mut cx);
        sleep_ms(100).await;
    }

    let peer_id_2 = peer_id_2.expect("Should get second connection");

    // === UNLINKABILITY VALIDATION ===

    console_log!("=== Unlinkability Validation ===");
    console_log!("PeerId from connection 1: {:?}", peer_id_1);
    console_log!("PeerId from connection 2: {:?}", peer_id_2);

    assert_ne!(
        peer_id_1, peer_id_2,
        "UNLINKABILITY VIOLATION: PeerIds should be different for each connection!"
    );

    console_log!("✓ PeerIds are different - connections are unlinkable!");
    console_log!("B cannot determine that both connections came from the same peer.");

    console_log!("=== test_07_peerid_unlinkability PASSED ===");

    // Cleanup
    drop(transport_a);
    drop(transport_b);
    sleep_ms(1000).await;
}

// Test 8: libp2p Ping Protocol - Full Swarm Integration

/// Test the full libp2p stack with the ping protocol.
///
/// This validates that:
/// - NymTransport works with libp2p Swarm
/// - The ping protocol can exchange messages over the mixnet
/// - Round-trip latency through the mixnet
#[wasm_bindgen_test]
async fn test_08_libp2p_ping() {
    use libp2p::swarm::SwarmEvent;
    use libp2p::{ping, SwarmBuilder};
    use std::time::Duration;

    console_log!("=== test_08_libp2p_ping ===");
    console_log!("Testing full libp2p swarm with ping protocol over mixnet...");

    // Create transport A
    let opts_a = TransportClientOpts {
        nym_api_url: None,
        force_tls: true,
        client_id: Some(format!("ping-a-{}", js_sys::Date::now() as u64)),
    };
    let result_a = create_transport_client_async(opts_a)
        .await
        .expect("Failed to create client A");
    let addr_a =
        nym_address_to_multiaddress(result_a.self_address).expect("Failed to create multiaddr A");

    console_log!("Created client A: {}", &addr_a.to_string()[..40]);

    // Create transport B
    let opts_b = TransportClientOpts {
        nym_api_url: None,
        force_tls: true,
        client_id: Some(format!("ping-b-{}", js_sys::Date::now() as u64)),
    };
    let result_b = create_transport_client_async(opts_b)
        .await
        .expect("Failed to create client B");
    let addr_b =
        nym_address_to_multiaddress(result_b.self_address).expect("Failed to create multiaddr B");

    console_log!("Created client B: {}", &addr_b.to_string()[..40]);

    // Build Swarm A
    let keypair_a = Keypair::generate_ed25519();
    let transport_a = NymTransport::new(result_a.self_address, result_a.stream, keypair_a.clone())
        .await
        .expect("Failed to create transport A");

    let mut swarm_a = SwarmBuilder::with_existing_identity(keypair_a)
        .with_wasm_bindgen()
        .with_other_transport(|_| Ok(transport_a))
        .expect("Failed to add transport A")
        .with_behaviour(|_| ping::Behaviour::default())
        .expect("Failed to add ping behaviour A")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(120)))
        .build();

    console_log!("Swarm A created with PeerId: {:?}", swarm_a.local_peer_id());

    // Build Swarm B
    let keypair_b = Keypair::generate_ed25519();
    let transport_b = NymTransport::new(result_b.self_address, result_b.stream, keypair_b.clone())
        .await
        .expect("Failed to create transport B");

    let mut swarm_b = SwarmBuilder::with_existing_identity(keypair_b)
        .with_wasm_bindgen()
        .with_other_transport(|_| Ok(transport_b))
        .expect("Failed to add transport B")
        .with_behaviour(|_| ping::Behaviour::default())
        .expect("Failed to add ping behaviour B")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(120)))
        .build();

    console_log!("Swarm B created with PeerId: {:?}", swarm_b.local_peer_id());

    // A dials B
    console_log!("Swarm A dialing Swarm B at {}", addr_b);
    swarm_a
        .dial(addr_b.clone())
        .expect("Dial should not fail immediately");

    // Poll both swarms waiting for ping events using futures::select!
    let start = js_sys::Date::now();
    let timeout = gloo_timers::future::TimeoutFuture::new(60_000).fuse();
    futures::pin_mut!(timeout);

    let ping_success = loop {
        select! {
            event = swarm_a.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(ping::Event { peer, result, .. }) => match result {
                        Ok(rtt) => {
                            console_log!("A: Ping to {:?} succeeded! RTT: {:?}", peer, rtt);
                            break true;
                        }
                        Err(e) => {
                            console_log!("A: Ping to {:?} failed: {:?}", peer, e);
                        }
                    },
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        console_log!("A: Connection established with {:?}", peer_id);
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        console_log!("A: Listening on {}", address);
                    }
                    _ => {}
                }
            },
            event = swarm_b.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(ping::Event { peer, result, .. }) => match result {
                        Ok(rtt) => {
                            console_log!("B: Ping to {:?} succeeded! RTT: {:?}", peer, rtt);
                            break true;
                        }
                        Err(e) => {
                            console_log!("B: Ping to {:?} failed: {:?}", peer, e);
                        }
                    },
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        console_log!("B: Connection established with {:?}", peer_id);
                    }
                    SwarmEvent::IncomingConnection {
                        local_addr,
                        send_back_addr,
                        ..
                    } => {
                        console_log!(
                            "B: Incoming connection from {} to {}",
                            send_back_addr,
                            local_addr
                        );
                    }
                    _ => {}
                }
            },
            _ = &mut timeout => {
                console_log!("Timeout waiting for ping!");
                break false;
            },
        }
    };

    let elapsed = (js_sys::Date::now() - start) / 1000.0;
    console_log!("Test completed after {:.1}s", elapsed);

    assert!(ping_success, "At least one ping should succeed");

    console_log!("=== test_08_libp2p_ping PASSED ===");
    console_log!("Successfully demonstrated libp2p ping over Nym mixnet!");

    // Cleanup
    sleep_ms(1000).await;
}

// Test 9: libp2p Identify Protocol

/// Test the libp2p identify protocol over the Nym mixnet.
///
/// This validates that peers can exchange identity information including
/// PeerId, protocol version, agent version, and supported protocols.
#[wasm_bindgen_test]
async fn test_09_libp2p_identify() {
    use libp2p::swarm::SwarmEvent;
    use libp2p::{identify, SwarmBuilder};
    use std::time::Duration;

    console_log!("=== test_09_libp2p_identify ===");
    console_log!("Testing libp2p identify protocol over mixnet...");

    // Create transport A
    let opts_a = TransportClientOpts {
        nym_api_url: None,
        force_tls: true,
        client_id: Some(format!("identify-a-{}", js_sys::Date::now() as u64)),
    };
    let result_a = create_transport_client_async(opts_a)
        .await
        .expect("Failed to create client A");
    let addr_a =
        nym_address_to_multiaddress(result_a.self_address).expect("Failed to create multiaddr A");

    console_log!("Created client A: {}", &addr_a.to_string()[..40]);

    // Create transport B
    let opts_b = TransportClientOpts {
        nym_api_url: None,
        force_tls: true,
        client_id: Some(format!("identify-b-{}", js_sys::Date::now() as u64)),
    };
    let result_b = create_transport_client_async(opts_b)
        .await
        .expect("Failed to create client B");
    let addr_b =
        nym_address_to_multiaddress(result_b.self_address).expect("Failed to create multiaddr B");

    console_log!("Created client B: {}", &addr_b.to_string()[..40]);

    // Build Swarm A with identify behaviour
    let keypair_a = Keypair::generate_ed25519();
    let transport_a = NymTransport::new(result_a.self_address, result_a.stream, keypair_a.clone())
        .await
        .expect("Failed to create transport A");

    let identify_config_a =
        identify::Config::new("/nym-test/1.0.0".to_string(), keypair_a.public())
            .with_agent_version("nym-libp2p-wasm-test/0.1.0".to_string());

    let mut swarm_a = SwarmBuilder::with_existing_identity(keypair_a)
        .with_wasm_bindgen()
        .with_other_transport(|_| Ok(transport_a))
        .expect("Failed to add transport A")
        .with_behaviour(|_| identify::Behaviour::new(identify_config_a))
        .expect("Failed to add identify behaviour A")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(120)))
        .build();

    console_log!("Swarm A created with PeerId: {:?}", swarm_a.local_peer_id());

    // Build Swarm B with identify behaviour
    let keypair_b = Keypair::generate_ed25519();
    let transport_b = NymTransport::new(result_b.self_address, result_b.stream, keypair_b.clone())
        .await
        .expect("Failed to create transport B");

    let identify_config_b =
        identify::Config::new("/nym-test/1.0.0".to_string(), keypair_b.public())
            .with_agent_version("nym-libp2p-wasm-test/0.1.0".to_string());

    let mut swarm_b = SwarmBuilder::with_existing_identity(keypair_b)
        .with_wasm_bindgen()
        .with_other_transport(|_| Ok(transport_b))
        .expect("Failed to add transport B")
        .with_behaviour(|_| identify::Behaviour::new(identify_config_b))
        .expect("Failed to add identify behaviour B")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(120)))
        .build();

    console_log!("Swarm B created with PeerId: {:?}", swarm_b.local_peer_id());

    // A dials B
    console_log!("Swarm A dialing Swarm B at {}", addr_b);
    swarm_a
        .dial(addr_b.clone())
        .expect("Dial should not fail immediately");

    // Poll both swarms waiting for identify events using futures::select!
    let start = js_sys::Date::now();
    let timeout = gloo_timers::future::TimeoutFuture::new(60_000).fuse();
    futures::pin_mut!(timeout);

    let identify_received = loop {
        select! {
            event = swarm_a.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(identify::Event::Received { peer_id, info, .. }) => {
                        console_log!("A: Received identify from {:?}", peer_id);
                        console_log!("   Protocol: {}", info.protocol_version);
                        console_log!("   Agent: {}", info.agent_version);
                        console_log!("   Protocols: {:?}", info.protocols);
                        break true;
                    }
                    SwarmEvent::Behaviour(identify::Event::Sent { peer_id, .. }) => {
                        console_log!("A: Sent identify to {:?}", peer_id);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        console_log!("A: Connection established with {:?}", peer_id);
                    }
                    _ => {}
                }
            },
            event = swarm_b.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(identify::Event::Received { peer_id, info, .. }) => {
                        console_log!("B: Received identify from {:?}", peer_id);
                        console_log!("   Protocol: {}", info.protocol_version);
                        console_log!("   Agent: {}", info.agent_version);
                        console_log!("   Protocols: {:?}", info.protocols);
                        break true;
                    }
                    SwarmEvent::Behaviour(identify::Event::Sent { peer_id, .. }) => {
                        console_log!("B: Sent identify to {:?}", peer_id);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        console_log!("B: Connection established with {:?}", peer_id);
                    }
                    SwarmEvent::IncomingConnection {
                        local_addr,
                        send_back_addr,
                        ..
                    } => {
                        console_log!(
                            "B: Incoming connection from {} to {}",
                            send_back_addr,
                            local_addr
                        );
                    }
                    _ => {}
                }
            },
            _ = &mut timeout => {
                console_log!("Timeout waiting for identify!");
                break false;
            },
        }
    };

    let elapsed = (js_sys::Date::now() - start) / 1000.0;
    console_log!("Test completed after {:.1}s", elapsed);

    assert!(
        identify_received,
        "At least one peer should receive identify info"
    );

    console_log!("=== test_09_libp2p_identify PASSED ===");
    console_log!("Successfully demonstrated libp2p identify over Nym mixnet!");

    // Cleanup
    sleep_ms(1000).await;
}
