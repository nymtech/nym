// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Packet bridge between the smoltcp device and the Nym mixnet.
//!
//! Two concurrent loops in one `spawn_local` task:
//!
//! **Outgoing** (smoltcp → mixnet): on each tick, drain the device tx queue
//! and send each IP packet to the IPR as an LP-framed DataRequest.
//!
//! **Incoming** (mixnet → smoltcp): receive `ReconstructedMessage` batches,
//! LP-decode, parse IPR responses, unbundle IP packets, push to device rx.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use nym_wasm_client_core::client::base_client::ClientInput;
use nym_wasm_client_core::Recipient;
use wasm_bindgen_futures::spawn_local;

use crate::ipr::{self, ReconstructedReceiver};
use crate::reactor::{ReactorNotify, SmoltcpStack};

/// Maximum outgoing packets sent per bridge tick.
///
/// Limits how long the bridge spends in the serial send loop before
/// returning to check for incoming messages. Remaining packets are
/// picked up on the next tick.
const MAX_OUTGOING_PER_TICK: usize = 8;

/// Start the bridge as a `spawn_local` background task.
///
/// Each iteration:
/// 1. Wait for an event (incoming message OR timer tick)
/// 2. Drain all pending incoming messages (non-blocking)
/// 3. Drain outgoing packets (capped at `MAX_OUTGOING_PER_TICK`)
///
/// Incoming is processed first to prevent starvation; the timer can
/// dominate `select!` if always ready.
#[allow(clippy::too_many_arguments)]
pub fn start_bridge(
    stack: Arc<Mutex<SmoltcpStack>>,
    client_input: Arc<ClientInput>,
    mut msg_receiver: ReconstructedReceiver,
    ipr_address: Recipient,
    stream_id: u64,
    seq: Arc<AtomicU32>,
    notify_reactor: ReactorNotify,
    shutdown: Arc<AtomicBool>,
    data_surbs: u32,
) {
    spawn_local(async move {
        let mut tx_interval = wasmtimer::tokio::interval(Duration::from_millis(5));

        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Block until something happens (incoming message or timer tick).
            // Incoming is listed first so it wins when both are ready.
            futures::select! {
                batch = msg_receiver.next().fuse() => {
                    let Some(messages) = batch else {
                        crate::util::debug_error!(
                            "[bridge] message channel closed"
                        );
                        break;
                    };
                    process_incoming(
                        &stack, &messages, stream_id, &notify_reactor,
                    );
                }
                _ = tx_interval.tick().fuse() => {}
            }

            // Non-blockingly drain any remaining incoming messages so we
            // never let them queue up while we're sending outgoing packets.
            while let Some(Some(messages)) = msg_receiver.next().now_or_never() {
                process_incoming(&stack, &messages, stream_id, &notify_reactor);
            }

            // Drain outgoing packets (capped to avoid starving incoming).
            let packets: Vec<Vec<u8>> = {
                let mut s = stack.lock().unwrap();
                s.device.drain_tx().take(MAX_OUTGOING_PER_TICK).collect()
            };

            if !packets.is_empty() {
                crate::util::debug_log!("[bridge] ▲ tx");
            }
            for packet in packets {
                let current_seq = seq.fetch_add(1, Ordering::Relaxed);
                if let Err(e) = ipr::send_ip_packet(
                    &client_input,
                    &ipr_address,
                    stream_id,
                    current_seq,
                    &packet,
                    data_surbs,
                )
                .await
                {
                    crate::util::debug_error!("[bridge] send error: {e}");
                }
            }
        }
    });
}

/// Process a batch of incoming mixnet messages: LP-decode, parse IPR
/// responses, push IP packets to the device rx queue, notify reactor.
fn process_incoming(
    stack: &Arc<Mutex<SmoltcpStack>>,
    messages: &[nym_wasm_client_core::ReconstructedMessage],
    stream_id: u64,
    notify_reactor: &ReactorNotify,
) {
    let mut pushed = 0usize;
    for msg in messages {
        match ipr::parse_incoming(msg, stream_id) {
            Ok(Some(packets)) => {
                let mut s = stack.lock().unwrap();
                for packet in &packets {
                    s.device.push_rx(packet.clone());
                    pushed += 1;
                }
            }
            Ok(None) => {}
            Err(e) => {
                crate::util::debug_error!("[bridge] incoming error: {e}");
            }
        }
    }

    if pushed > 0 {
        crate::util::debug_log!("[bridge] ▼ rx");
        let _ = notify_reactor.unbounded_send(());
    }
}
