// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! IPR (IP Packet Router) protocol layer for the WASM tunnel.
//!
//! Handles the v9 connect handshake and IP packet send/recv, using the
//! upstream `nym_lp_data::packet::frame` wire format directly (no tokio deps).
//!
//! Data flow:
//! ```text
//! Outgoing: IP packet → bundle → DataRequest → to_bytes → LP frame → mixnet
//! Incoming: mixnet → LP decode → IpPacketResponse → unbundle → IP packets
//! ```

use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use futures::channel::mpsc;
use std::sync::Arc;
use std::time::Duration;

use nym_ip_packet_requests::IpPair;
use nym_ip_packet_requests::v9::{self, response::IpPacketResponse};
use nym_ip_packet_requests::v10::{self, response::IpPacketResponse as IpPacketResponseV10};
use nym_lp_data::packet::frame::{
    LpFrame, LpFrameKind, SphinxStreamFrameAttributes, SphinxStreamMsgType,
};
use nym_wasm_client_core::Recipient;
use nym_wasm_client_core::ReconstructedMessage;
use nym_wasm_client_core::client::base_client::ClientInput;
use nym_wasm_client_core::client::inbound_messages::InputMessage;
use nym_wasm_client_core::nym_task::connections::TransmissionLane;

use crate::error::FetchError;

/// Reply-SURB counts for the Open and Data frames. Defaults: `open=10, data=2`.
///
/// `open` seeds the IPR's SURB bucket on the connect handshake. `data` is the
/// number of reply-SURBs attached to every packet we send (including TCP ACKs);
/// it funds the IPR's return traffic for the connection.
///
/// `data` is deliberately small, and raising it has a cost that is easy to
/// miss. A reply-SURB is not a flag on the packet: it is a full layer-encrypted
/// return header that travels as forward payload, and each Sphinx packet has a fixed
/// payload budget.
///
/// Return capacity for downloads does not need a large `data`: every ACK we send
/// during a transfer carries `data` SURBs, so capacity scales with the ACK rate
/// (which scales with the download rate), and the reply controller's pre-emptive
/// topup refills the bucket besides.
#[derive(Clone, Copy)]
pub struct SurbsConfig {
    pub open: u32,
    pub data: u32,
}

impl Default for SurbsConfig {
    fn default() -> Self {
        Self { open: 10, data: 2 }
    }
}

/// Type alias for the channel receiving batches of reconstructed messages.
pub type ReconstructedReceiver = mpsc::UnboundedReceiver<Vec<ReconstructedMessage>>;

/// Cap on the v10 connect attempt when a v9 fallback is still reachable, so a
/// node whose directory version leads its running process (mid-rollout) falls
/// back promptly instead of after the full `connect_timeout`. Generously above a
/// healthy v10 connect so a merely-slow node isn't downgraded spuriously.
const IPR_V10_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

/// Open an LP stream and run the connect handshake: Open frame, then a single
/// connect (Data seq=0) at the protocol version the node's release supports,
/// chosen from its directory version (`None` = unknown node ⇒ v9). On a v10
/// connect timeout it retries v9 once, in case the directory version leads the
/// running IPR. Returns the allocated IPs and, on v10, the reported MTU.
pub async fn open_and_connect(
    client_input: &Arc<ClientInput>,
    receiver: &mut ReconstructedReceiver,
    ipr_address: &Recipient,
    stream_id: u64,
    surbs: SurbsConfig,
    connect_timeout: Duration,
    node_version: Option<&semver::Version>,
) -> Result<(IpPair, Option<u16>), FetchError> {
    nym_wasm_utils::console_log!("[ipr] sending connect handshake...");
    crate::util::debug_log!("[ipr] stream={stream_id:#018x}");

    // 1. Send LP Open frame (empty payload, seq=0); establishes the stream.
    // Data frames have their own seq space; Open's seq field is independent.
    let open_frame = encode_lp_frame(stream_id, SphinxStreamMsgType::Open, 0, &[]);
    send_to_ipr(client_input, ipr_address, open_frame, surbs.open).await?;

    // 2. Connect (Data seq=0) at the version the node's release supports.
    if node_version.is_none() {
        crate::util::debug_log!(
            "[ipr] no directory version for node; connecting v9 (MTU unreported)"
        );
    }
    // Fall through to the v9 connect below unless we settle on v10. A v10 timeout
    // also falls through, since the directory version can lead the running IPR.
    match node_version.and_then(nym_ip_packet_requests::best_supported_version) {
        Some(v) if v == nym_ip_packet_requests::v10::VERSION => {
            // Bound the v10 attempt so the v9 fallback below stays responsive.
            let v10_timeout = connect_timeout.min(IPR_V10_ATTEMPT_TIMEOUT);
            match connect_v10(
                client_input,
                receiver,
                ipr_address,
                stream_id,
                0,
                surbs,
                v10_timeout,
            )
            .await
            {
                Ok(success) => return Ok((success.ips, Some(success.mtu))),
                // Directory version can lead the running IPR; retry v9. seq=0 again is
                // safe, since the IPR doesn't dedup uplink frames by seq.
                Err(FetchError::Timeout) => {
                    crate::util::debug_log!("[ipr] v10 connect timed out; retrying v9");
                }
                Err(e) => return Err(e),
            }
        }
        Some(v) if v == nym_ip_packet_requests::v9::VERSION => {}
        Some(other) => {
            crate::util::debug_log!(
                "[ipr] node advertises unsupported IPR version v{other}; connecting v9"
            );
        }
        None => {} // unknown node, logged above
    }
    let ips = connect_v9(
        client_input,
        receiver,
        ipr_address,
        stream_id,
        0,
        surbs,
        connect_timeout,
    )
    .await?;
    Ok((ips, None))
}

/// v10 connect: Data frame at `seq`; returns the IPR's MTU + allocated IPs.
async fn connect_v10(
    client_input: &Arc<ClientInput>,
    receiver: &mut ReconstructedReceiver,
    ipr_address: &Recipient,
    stream_id: u64,
    seq: u32,
    surbs: SurbsConfig,
    connect_timeout: Duration,
) -> Result<nym_ip_packet_requests::v10::response::ConnectSuccess, FetchError> {
    let (request, request_id) = v10::new_connect_request(None);
    let request_bytes = request
        .to_bytes()
        .map_err(|e| FetchError::Tunnel(format!("failed to serialise v10 connect request: {e}")))?;
    let data_frame = encode_lp_frame(stream_id, SphinxStreamMsgType::Data, seq, &request_bytes);
    send_to_ipr(client_input, ipr_address, data_frame, surbs.data).await?;

    wasmtimer::tokio::timeout(connect_timeout, async {
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

                // Ignore stragglers from an earlier version; we selected v10 from
                // the node's directory version.
                if content.first() != Some(&v10::VERSION) {
                    continue;
                }

                let response = match IpPacketResponseV10::from_bytes(&content) {
                    Ok(r) => r,
                    Err(e) => {
                        crate::util::debug_error!(
                            "[ipr] malformed v10 response on our stream (dropped): {e}"
                        );
                        continue;
                    }
                };
                if response.id() != Some(request_id) {
                    continue;
                }
                return nym_ip_packet_requests::response_helpers::parse_connect_response_v10(
                    response,
                )
                .map_err(|e| FetchError::Tunnel(format!("IPR connect denied: {e}")));
            }
        }
    })
    .await
    .map_err(|_| FetchError::Timeout)?
}

/// v9 connect: pre-v10 path, no MTU reported.
async fn connect_v9(
    client_input: &Arc<ClientInput>,
    receiver: &mut ReconstructedReceiver,
    ipr_address: &Recipient,
    stream_id: u64,
    seq: u32,
    surbs: SurbsConfig,
    connect_timeout: Duration,
) -> Result<IpPair, FetchError> {
    let (request, request_id) = v9::new_connect_request(None);
    let request_bytes = request
        .to_bytes()
        .map_err(|e| FetchError::Tunnel(format!("failed to serialise connect request: {e}")))?;
    let data_frame = encode_lp_frame(stream_id, SphinxStreamMsgType::Data, seq, &request_bytes);
    send_to_ipr(client_input, ipr_address, data_frame, surbs.data).await?;

    wasmtimer::tokio::timeout(connect_timeout, async {
        loop {
            let batch = receiver
                .next()
                .await
                .ok_or_else(|| FetchError::Tunnel("message channel closed".into()))?;

            for msg in batch {
                // nym-client-core's received_buffer filters cover traffic
                // before delivery, so an outer LP-decode failure here is a
                // "shouldn't happen" signal: either a non-LP straggler or
                // garbage. We log and continue rather than bail (bailing
                // would open a single-spoofed-message DoS on the handshake);
                // tightening to fail-fast belongs with the IPR-auth design.
                let Some((attrs, content)) = decode_lp_stream(&msg.message) else {
                    crate::util::debug_error!(
                        "[ipr] non-LP-stream message received during handshake (dropped)"
                    );
                    continue;
                };

                if attrs.stream_id != stream_id || attrs.msg_type != SphinxStreamMsgType::Data {
                    // Late straggler from a different stream/session — expected.
                    continue;
                }

                let response = match IpPacketResponse::from_bytes(&content) {
                    Ok(r) => r,
                    Err(e) => {
                        crate::util::debug_error!(
                            "[ipr] malformed IpPacketResponse on our stream (dropped): {e}"
                        );
                        continue;
                    }
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
    .map_err(|_| FetchError::Timeout)?
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
    data_surbs: u32,
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
    send_to_ipr(client_input, ipr_address, frame, data_surbs).await
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
            crate::util::debug_error!("[ipr] IPR sent DISCONNECT");
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
async fn send_to_ipr(
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

/// Performance-weighted random pick from v9-capable IPRs. Ported from
/// `nym_sdk::ip_packet_client::discovery::get_best_ipr` to keep the
/// SDK out of the wasm dep graph.
pub(crate) async fn discover_ipr(
    nym_api_urls: &[url::Url],
) -> Result<(Recipient, semver::Version), FetchError> {
    use nym_validator_client::nym_api::NymApiClientExt;
    use rand::seq::SliceRandom;
    use std::collections::HashMap;

    let url = nym_api_urls
        .first()
        .ok_or_else(|| FetchError::Tunnel("no nym-api URLs for IPR discovery".into()))?;
    let client = nym_wasm_client_core::ApiClient::builder(url.clone())
        .map_err(|e| FetchError::Tunnel(format!("nym-api builder failed: {e}")))?
        .build()
        .map_err(|e| FetchError::Tunnel(format!("nym-api build failed: {e}")))?;

    let all_nodes: HashMap<_, _> = client
        .get_all_described_nodes_v2()
        .await
        .map_err(|e| FetchError::Tunnel(format!("describe nodes failed: {e}")))?
        .into_iter()
        .map(|d| (d.ed25519_identity_key(), d))
        .collect();

    let exits = client
        .get_all_basic_nodes_with_metadata()
        .await
        .map_err(|e| FetchError::Tunnel(format!("list nodes failed: {e}")))?
        .nodes;

    let mut candidates: Vec<(Recipient, u8, semver::Version)> = Vec::new();
    for exit in exits {
        // We fetch all nodes above, then keep only those declaring the exit-IPR
        // role (an exit gateway can be network-requester-only); others don't serve
        // an IPR, so a v9 connect to them just times out.
        //
        // TODO(ipr-perf): rewrite this selection when IPR performance monitoring
        // lands — fetch only exit gateways and rank on measured IPR health, since
        // directory `performance` doesn't predict IPR usability.
        if !exit.supported_roles.exit_ipr {
            continue;
        }
        let Some(node) = all_nodes.get(&exit.ed25519_identity_pubkey) else {
            continue;
        };
        let Ok(version) = semver::Version::parse(node.version()) else {
            continue;
        };
        if version < nym_ip_packet_requests::v9::MIN_RELEASE_VERSION {
            continue;
        }
        let Some(ipr_info) = node.description.ip_packet_router.clone() else {
            continue;
        };
        let Ok(addr) = ipr_info.address.parse::<Recipient>() else {
            continue;
        };
        candidates.push((addr, exit.performance.round_to_integer(), version));
    }

    let picked = candidates
        .choose_weighted(&mut rand::thread_rng(), |c| c.1 as f64)
        .map_err(|_| FetchError::Tunnel("no v9-capable IPRs available".into()))?;
    nym_wasm_utils::console_log!(
        "[smolmix] auto-discovered IPR (v{}): {}",
        picked.2,
        picked.0
    );
    Ok((picked.0, picked.2.clone()))
}

/// Look up a node's release version by its identity (the gateway in an IPR
/// address), so an explicit-address connect can pick the protocol version from
/// the directory too.
pub(crate) async fn lookup_node_version(
    nym_api_urls: &[url::Url],
    ipr_address: &Recipient,
) -> Result<semver::Version, FetchError> {
    use nym_validator_client::nym_api::NymApiClientExt;

    let url = nym_api_urls
        .first()
        .ok_or_else(|| FetchError::Tunnel("no nym-api URLs for version lookup".into()))?;
    let client = nym_wasm_client_core::ApiClient::builder(url.clone())
        .map_err(|e| FetchError::Tunnel(format!("nym-api builder failed: {e}")))?
        .build()
        .map_err(|e| FetchError::Tunnel(format!("nym-api build failed: {e}")))?;

    let gateway = ipr_address.gateway();
    let node = client
        .get_all_described_nodes_v2()
        .await
        .map_err(|e| FetchError::Tunnel(format!("describe nodes failed: {e}")))?
        .into_iter()
        .find(|n| n.ed25519_identity_key() == gateway)
        .ok_or_else(|| FetchError::Tunnel("IPR node not found in directory".into()))?;
    semver::Version::parse(node.version())
        .map_err(|e| FetchError::Tunnel(format!("bad node version: {e}")))
}
