// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! IPR (IP Packet Router) protocol layer for the WASM tunnel.
//!
//! Handles the v9 connect handshake and IP packet send/recv, using our local
//! LP framing (lp.rs) instead of `MixnetStream` (which depends on tokio).
//!
//! Data flow:
//! ```text
//! Outgoing: IP packet → bundle → DataRequest → to_bytes → LP frame → mixnet
//! Incoming: mixnet → LP decode → IpPacketResponse → unbundle → IP packets
//! ```
//!
//! Reference: `sdk/rust/nym-sdk/src/ipr_wrapper/ip_mix_stream.rs`

use bytes::Bytes;
use futures::channel::mpsc;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;

use nym_ip_packet_requests::v9::{self, response::IpPacketResponse};
use nym_ip_packet_requests::IpPair;
use nym_wasm_client_core::client::base_client::ClientInput;
use nym_wasm_client_core::client::inbound_messages::InputMessage;
use nym_wasm_client_core::nym_task::connections::TransmissionLane;
use nym_wasm_client_core::Recipient;
use nym_wasm_client_core::ReconstructedMessage;

use crate::error::FetchError;
use crate::lp;

/// SURBs attached to the LP Open frame (establishes reply capability).
const OPEN_REPLY_SURBS: u32 = 10;

/// SURBs attached to each data message (replenishes reply pool).
const DATA_REPLY_SURBS: u32 = 2;

/// Timeout for the IPR connect handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

/// Type alias for the channel receiving batches of reconstructed messages.
pub type ReconstructedReceiver = mpsc::UnboundedReceiver<Vec<ReconstructedMessage>>;

/// Open an LP stream to the IPR and perform the v9 connect handshake.
///
/// Sends an LP Open frame (seq=0, empty payload), then a ConnectRequest
/// (seq=1), and waits for a ConnectSuccess response with allocated IPs.
pub async fn open_and_connect(
    client_input: &Arc<ClientInput>,
    receiver: &mut ReconstructedReceiver,
    ipr_address: &Recipient,
    stream_id: u64,
) -> Result<IpPair, FetchError> {
    nym_wasm_utils::console_log!("[ipr] sending connect handshake (stream={stream_id:#018x})...");

    // 1. Send LP Open frame (empty payload, seq=0) — establishes the stream
    let open_frame = lp::encode(stream_id, lp::MsgType::Open, 0, &[]);
    send_anonymous(client_input, ipr_address, open_frame, OPEN_REPLY_SURBS).await?;

    // 2. Send v9 ConnectRequest as LP Data frame (seq=0).
    //
    // Data frames start at seq=0 — the Open frame's seq field is NOT part
    // of the Data sequence space.  The receiver's reorder buffer only tracks
    // Data frames and expects the first one at seq=0.  This matches native
    // MixnetStream, which initialises next_seq=0 independently of the Open.
    let (request, request_id) = v9::new_connect_request(None);
    let request_bytes = request
        .to_bytes()
        .map_err(|e| FetchError::Tunnel(format!("failed to serialise connect request: {e}")))?;
    let data_frame = lp::encode(stream_id, lp::MsgType::Data, 0, &request_bytes);
    send_anonymous(client_input, ipr_address, data_frame, DATA_REPLY_SURBS).await?;

    // 3. Wait for ConnectSuccess response
    let ip_pair = wasmtimer::tokio::timeout(CONNECT_TIMEOUT, async {
        loop {
            let batch = receiver
                .next()
                .await
                .ok_or_else(|| FetchError::Tunnel("message channel closed".into()))?;

            for msg in batch {
                let Some(frame) = lp::decode(&msg.message) else {
                    continue;
                };

                if frame.stream_id != stream_id || frame.msg_type != lp::MsgType::Data {
                    continue;
                }

                let response = match IpPacketResponse::from_bytes(&frame.payload) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                if response.id() != Some(request_id) {
                    continue;
                }

                return nym_ip_packet_requests::response_helpers::parse_connect_response(response)
                    .map_err(|e| FetchError::Tunnel(format!("IPR connect denied: {e}")));
            }
        }
    })
    .await
    .map_err(|_| FetchError::Tunnel("IPR connect timed out".into()))??;

    Ok(ip_pair)
}

/// Bundle an IP packet and send it to the IPR as an LP-framed DataRequest.
///
/// The bundling uses the `MultiIpPacketCodec` wire format: 2-byte BE length
/// prefix followed by the raw packet. This is what the IPR expects.
pub async fn send_ip_packet(
    client_input: &Arc<ClientInput>,
    ipr_address: &Recipient,
    stream_id: u64,
    seq: u32,
    packet: &[u8],
) -> Result<(), FetchError> {
    let bundled = bundle_one_packet(packet);

    // Wrap in v9 DataRequest
    let request = v9::new_data_request(bundled);
    let request_bytes = request
        .to_bytes()
        .map_err(|e| FetchError::Tunnel(format!("failed to serialise data request: {e}")))?;

    // LP-frame and send
    let frame = lp::encode(stream_id, lp::MsgType::Data, seq, &request_bytes);
    send_anonymous(client_input, ipr_address, frame, DATA_REPLY_SURBS).await
}

/// Parse an incoming ReconstructedMessage into individual IP packets.
///
/// LP-decodes the message, verifies the stream_id, deserialises the IPR
/// response, and unbundles the contained IP packets.
///
/// Returns `Ok(None)` for non-data responses (control messages, wrong stream).
/// Returns `Ok(Some(packets))` for data responses.
/// Returns `Err` only for hard errors (disconnect).
pub fn parse_incoming(
    msg: &ReconstructedMessage,
    expected_stream_id: u64,
) -> Result<Option<Vec<Vec<u8>>>, FetchError> {
    let Some(frame) = lp::decode(&msg.message) else {
        return Ok(None);
    };

    if frame.stream_id != expected_stream_id || frame.msg_type != lp::MsgType::Data {
        return Ok(None);
    }

    match nym_ip_packet_requests::response_helpers::handle_ipr_response(&frame.payload) {
        Some(nym_ip_packet_requests::response_helpers::MixnetMessageOutcome::IpPackets(
            packets,
        )) => Ok(Some(packets.into_iter().map(|b| b.to_vec()).collect())),
        Some(nym_ip_packet_requests::response_helpers::MixnetMessageOutcome::Disconnect) => {
            nym_wasm_utils::console_error!("[ipr] IPR sent DISCONNECT");
            Err(FetchError::Tunnel("IPR disconnected".into()))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Send an anonymous mixnet message to the IPR with reply SURBs.
async fn send_anonymous(
    client_input: &Arc<ClientInput>,
    recipient: &Recipient,
    data: Vec<u8>,
    reply_surbs: u32,
) -> Result<(), FetchError> {
    let msg = InputMessage::new_anonymous(
        *recipient,
        data,
        reply_surbs,
        TransmissionLane::General,
        None,
    );
    client_input
        .send(msg)
        .await
        .map_err(|_| FetchError::Tunnel("mixnet input channel closed".into()))
}

/// Bundle a single IP packet in `MultiIpPacketCodec` format.
///
/// Wire format: `[len: u16 BE][packet bytes]`
///
/// Reimplemented locally to avoid depending on `tokio_util::codec` traits
/// (the `MultiIpPacketCodec::bundle_one_packet` static method does exactly
/// this, but the crate's codec module pulls in tokio-util).
fn bundle_one_packet(packet: &[u8]) -> Bytes {
    let mut buf = Vec::with_capacity(2 + packet.len());
    buf.extend_from_slice(&(packet.len() as u16).to_be_bytes());
    buf.extend_from_slice(packet);
    Bytes::from(buf)
}
