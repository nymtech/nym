// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::{Bytes, BytesMut};
use nym_ip_packet_requests::{codec::MultiIpPacketCodec, v8::response::ControlResponse};
use tokio_util::codec::Decoder;
use tracing::{error, info, warn};

use crate::ip_packet_client::current::{
    response::{InfoLevel, IpPacketResponse, IpPacketResponseData},
    VERSION as CURRENT_VERSION,
};

pub enum MixnetMessageOutcome {
    IpPackets(Vec<Bytes>),
    Disconnect,
}

/// Check that the first byte of an IPR message matches the expected protocol version.
pub(crate) fn check_ipr_message_version(data: &[u8]) -> Result<(), crate::Error> {
    let version = data.first().ok_or_else(|| {
        crate::Error::IPRMessageVersionCheckFailed("no version byte in message".into())
    })?;
    if *version != CURRENT_VERSION {
        return Err(crate::Error::IPRMessageVersionCheckFailed(format!(
            "received v{version}, expected v{CURRENT_VERSION}"
        )));
    }
    Ok(())
}

/// Parse raw IPR response bytes into an outcome.
pub fn handle_ipr_response(data: &[u8]) -> Result<Option<MixnetMessageOutcome>, crate::Error> {
    check_ipr_message_version(data)?;

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
                return Ok(Some(MixnetMessageOutcome::IpPackets(packets)));
            }
            IpPacketResponseData::Control(control_response) => match *control_response {
                ControlResponse::Connect(_) => {
                    info!("Received connect response when already connected - ignoring");
                }
                ControlResponse::Disconnect(_) | ControlResponse::UnrequestedDisconnect(_) => {
                    info!("Received disconnect from IPR");
                    return Ok(Some(MixnetMessageOutcome::Disconnect));
                }
                ControlResponse::Pong(_) => {
                    info!("Received pong response");
                }
                ControlResponse::Health(_) => {
                    info!("Received health response");
                }
                ControlResponse::Info(info) => {
                    let msg = format!("Received info response from the mixnet: {}", info.reply);
                    match info.level {
                        InfoLevel::Info => info!("{msg}"),
                        InfoLevel::Warn => warn!("{msg}"),
                        InfoLevel::Error => error!("{msg}"),
                    }
                }
            },
        },
        Err(err) => {
            warn!("Failed to deserialize IPR response: {err}");
        }
    }
    Ok(None)
}
