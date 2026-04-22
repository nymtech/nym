// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;
use tracing::{debug, error, info, warn};

use crate::{
    IpPair,
    codec::MultiIpPacketCodec,
    v8::response::{
        ConnectResponseReply, ControlResponse, InfoLevel, IpPacketResponse, IpPacketResponseData,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum IprResponseError {
    #[error("no version byte in message")]
    NoVersionByte,

    #[error("version mismatch: received v{received}, expected v{expected}")]
    VersionMismatch { expected: u8, received: u8 },

    #[error("expected control response, got {0:?}")]
    UnexpectedResponse(IpPacketResponseData),

    #[error("connect denied: {0:?}")]
    ConnectDenied(crate::v8::response::ConnectFailureReason),
}

pub enum MixnetMessageOutcome {
    IpPackets(Vec<Bytes>),
    Disconnect,
}

// Extracted from:
//   nym-ip-packet-client/src/helpers.rs — check_ipr_message_version()
//   sdk/rust/nym-sdk/src/ip_packet_client/listener.rs — check_ipr_message_version()
/// Check that the first byte of an IPR message matches the expected protocol version.
///
/// v9 currently reuses the v8 bincode layout (`nym_ip_packet_requests::v9` re-exports v8 types);
/// the version byte signals LP/SphinxStream framing, not a wire-format change. Until exit gateways
/// have rolled past `crate::v9::MIN_RELEASE_VERSION`, a v9 client may still receive v8 replies and
/// must accept them. Revisit this compat branch if a future bump diverges the wire layout.
///
/// TODO(IPR-v9-rollout): remove the v9-accepts-v8 branch once the exit gateway fleet is on
/// `crate::v9::MIN_RELEASE_VERSION` or newer.
pub fn check_ipr_message_version(data: &[u8], expected: u8) -> Result<(), IprResponseError> {
    let version = *data.first().ok_or(IprResponseError::NoVersionByte)?;
    if version == expected {
        return Ok(());
    }
    if expected == crate::v9::VERSION && version == crate::v8::VERSION {
        debug!(
            "accepting v{} IPR reply under v{} client compat",
            crate::v8::VERSION,
            crate::v9::VERSION
        );
        return Ok(());
    }
    Err(IprResponseError::VersionMismatch {
        expected,
        received: version,
    })
}

// Extracted from:
//   nym-ip-packet-client/src/connect.rs — handle_connect_response() + handle_ip_packet_router_response()
//   sdk/rust/nym-sdk/src/ip_packet_client/discovery.rs — parse_connect_response()
/// Parse an IPR connect response, returning allocated IPs on success.
pub fn parse_connect_response(response: IpPacketResponse) -> Result<IpPair, IprResponseError> {
    let control_response = match response.data {
        IpPacketResponseData::Control(c) => c,
        other => return Err(IprResponseError::UnexpectedResponse(other)),
    };

    match *control_response {
        ControlResponse::Connect(connect_resp) => match connect_resp.reply {
            ConnectResponseReply::Success(success) => Ok(success.ips),
            ConnectResponseReply::Failure(reason) => Err(IprResponseError::ConnectDenied(reason)),
        },
        _ => Err(IprResponseError::UnexpectedResponse(
            IpPacketResponseData::Control(control_response),
        )),
    }
}

// Extracted from:
//   nym-ip-packet-client/src/listener.rs — IprListener::handle_reconstructed_message()
//   sdk/rust/nym-sdk/src/ip_packet_client/listener.rs — handle_ipr_response()
/// Parse raw IPR response bytes into an outcome.
///
/// Logs non-fatal conditions (unknown control messages, deserialization
/// failures) and returns `None` for them.
pub fn handle_ipr_response(data: &[u8]) -> Option<MixnetMessageOutcome> {
    match IpPacketResponse::from_bytes(data) {
        Ok(response) => match response.data {
            IpPacketResponseData::Data(data_response) => {
                let mut codec = MultiIpPacketCodec::new();
                let mut buf = BytesMut::from(data_response.ip_packet.as_ref());
                let mut packets = Vec::new();
                loop {
                    match codec.decode(&mut buf) {
                        Ok(Some(packet)) => packets.push(packet.into_bytes()),
                        Ok(None) => break,
                        Err(e) => {
                            warn!("Failed to decode bundled IP packet: {e}");
                            break;
                        }
                    }
                }
                Some(MixnetMessageOutcome::IpPackets(packets))
            }
            IpPacketResponseData::Control(control_response) => match *control_response {
                ControlResponse::Connect(_) => {
                    info!("Received connect response when already connected - ignoring");
                    None
                }
                ControlResponse::Disconnect(_) | ControlResponse::UnrequestedDisconnect(_) => {
                    info!("Received disconnect from IPR");
                    Some(MixnetMessageOutcome::Disconnect)
                }
                ControlResponse::Pong(_) => {
                    info!("Received pong response");
                    None
                }
                ControlResponse::Health(_) => {
                    info!("Received health response");
                    None
                }
                ControlResponse::Info(info_resp) => {
                    let msg = format!(
                        "Received info response from the mixnet: {}",
                        info_resp.reply
                    );
                    match info_resp.level {
                        InfoLevel::Info => info!("{msg}"),
                        InfoLevel::Warn => warn!("{msg}"),
                        InfoLevel::Error => error!("{msg}"),
                    }
                    None
                }
            },
        },
        Err(err) => {
            warn!("Failed to deserialize IPR response: {err}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_version_matches() {
        assert!(check_ipr_message_version(&[crate::v9::VERSION], crate::v9::VERSION).is_ok());
        assert!(check_ipr_message_version(&[crate::v8::VERSION], crate::v8::VERSION).is_ok());
    }

    #[test]
    fn v9_client_accepts_v8_reply_compat() {
        assert!(check_ipr_message_version(&[crate::v8::VERSION], crate::v9::VERSION).is_ok());
    }

    #[test]
    fn v8_client_rejects_v9_reply() {
        let err = check_ipr_message_version(&[crate::v9::VERSION], crate::v8::VERSION)
            .expect_err("v8 client must not silently accept v9");
        assert!(matches!(
            err,
            IprResponseError::VersionMismatch {
                expected: 8,
                received: 9
            }
        ));
    }

    #[test]
    fn rejects_unrelated_version_mismatch() {
        let err = check_ipr_message_version(&[7], crate::v9::VERSION)
            .expect_err("v9 client must reject v7");
        assert!(matches!(
            err,
            IprResponseError::VersionMismatch {
                expected: 9,
                received: 7
            }
        ));
    }

    #[test]
    fn empty_payload_returns_no_version_byte() {
        let err = check_ipr_message_version(&[], crate::v9::VERSION)
            .expect_err("empty payload must error");
        assert!(matches!(err, IprResponseError::NoVersionByte));
    }
}
