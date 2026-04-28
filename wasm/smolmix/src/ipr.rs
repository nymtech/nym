// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

//! IPR (IP Packet Router) protocol layer for the WASM tunnel.
//!
//! Handles the v9 connect handshake and IP packet send/recv, using the
//! upstream `nym_lp::packet::frame` wire format directly (no tokio deps).
//!
//! Data flow:
//! ```text
//! Outgoing: IP packet → bundle → DataRequest → to_bytes → LP frame → mixnet
//! Incoming: mixnet → LP decode → IpPacketResponse → unbundle → IP packets
//! ```
//!
//! Reference: `sdk/rust/nym-sdk/src/ipr_wrapper/ip_mix_stream.rs`

use bytes::{Bytes, BytesMut};
use futures::channel::mpsc;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;

use nym_ip_packet_requests::v9::{self, response::IpPacketResponse};
use nym_ip_packet_requests::IpPair;
use nym_lp::packet::frame::{
    LpFrame, LpFrameKind, SphinxStreamFrameAttributes, SphinxStreamMsgType,
};
use nym_wasm_client_core::client::base_client::ClientInput;
use nym_wasm_client_core::client::inbound_messages::InputMessage;
use nym_wasm_client_core::nym_task::connections::TransmissionLane;
use nym_wasm_client_core::Recipient;
use nym_wasm_client_core::ReconstructedMessage;

use crate::error::FetchError;

/// SURBs for the LP Open frame — higher than DATA so the IPR has a reply
/// pool before data flows back.
const OPEN_REPLY_SURBS: u32 = 5;

/// SURBs per data message (replenishes reply pool).
///
/// Trade-off: fewer SURBs = smaller outgoing packets (faster sends),
/// but limits download throughput (IPR can only reply with N packets).
/// 2 fits in 1 Sphinx packet with minimal overhead.
const DATA_REPLY_SURBS: u32 = 2;

/// Timeout for the IPR connect handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

/// Type alias for the channel receiving batches of reconstructed messages.
pub type ReconstructedReceiver = mpsc::UnboundedReceiver<Vec<ReconstructedMessage>>;

/// Open an LP stream to the IPR and perform the v9 connect handshake.
///
/// Sends an LP Open frame (seq=0, empty payload), then a ConnectRequest
/// (Data seq=0), and waits for a ConnectSuccess response with allocated IPs.
pub async fn open_and_connect(
    client_input: &Arc<ClientInput>,
    receiver: &mut ReconstructedReceiver,
    ipr_address: &Recipient,
    stream_id: u64,
) -> Result<IpPair, FetchError> {
    nym_wasm_utils::console_log!("[ipr] sending connect handshake (stream={stream_id:#018x})...");

    // 1. Send LP Open frame (empty payload, seq=0) — establishes the stream
    let open_frame = encode_lp_frame(stream_id, SphinxStreamMsgType::Open, 0, &[]);
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
    let data_frame = encode_lp_frame(stream_id, SphinxStreamMsgType::Data, 0, &request_bytes);
    send_anonymous(client_input, ipr_address, data_frame, DATA_REPLY_SURBS).await?;

    // 3. Wait for ConnectSuccess response
    let ip_pair = wasmtimer::tokio::timeout(CONNECT_TIMEOUT, async {
        loop {
            let batch = receiver
                .next()
                .await
                .ok_or_else(|| FetchError::Tunnel("message channel closed".into()))?;

            for msg in batch {
                let Some((attrs, content)) = decode_lp_stream(&msg.message) else {
                    continue;
                };

                if attrs.stream_id != stream_id || attrs.msg_type != SphinxStreamMsgType::Data {
                    continue;
                }

                let response = match IpPacketResponse::from_bytes(&content) {
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
    let bundled = nym_ip_packet_requests::codec::MultiIpPacketCodec::bundle_one_packet(
        Bytes::copy_from_slice(packet),
    );

    // Wrap in v9 DataRequest
    let request = v9::new_data_request(bundled);
    let request_bytes = request
        .to_bytes()
        .map_err(|e| FetchError::Tunnel(format!("failed to serialise data request: {e}")))?;

    // LP-frame and send
    let frame = encode_lp_frame(stream_id, SphinxStreamMsgType::Data, seq, &request_bytes);
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
    let Some((attrs, content)) = decode_lp_stream(&msg.message) else {
        return Ok(None);
    };

    if attrs.stream_id != expected_stream_id || attrs.msg_type != SphinxStreamMsgType::Data {
        return Ok(None);
    }

    match nym_ip_packet_requests::response_helpers::handle_ipr_response(&content) {
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

// LP frame helpers

/// Encode a SphinxStream LP frame into bytes.
fn encode_lp_frame(
    stream_id: u64,
    msg_type: SphinxStreamMsgType,
    seq: u32,
    payload: &[u8],
) -> Vec<u8> {
    let frame = LpFrame::new_stream(
        SphinxStreamFrameAttributes {
            stream_id,
            msg_type,
            sequence_num: seq,
        },
        payload.to_vec(),
    );
    let mut buf = BytesMut::with_capacity(16 + payload.len());
    frame.encode(&mut buf);
    buf.to_vec()
}

/// Decode a SphinxStream LP frame, returning (attributes, content).
///
/// Returns `None` if the data is too short, the frame kind isn't
/// `SphinxStream`, or the attributes can't be parsed.
fn decode_lp_stream(data: &[u8]) -> Option<(SphinxStreamFrameAttributes, Bytes)> {
    let frame = LpFrame::decode(data).ok()?;
    if frame.kind() != LpFrameKind::SphinxStream {
        return None;
    }
    let attrs = SphinxStreamFrameAttributes::parse(&frame.header.frame_attributes).ok()?;
    Some((attrs, frame.content))
}

// Mixnet send helper

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
