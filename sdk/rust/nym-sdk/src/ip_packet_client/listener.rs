// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use bytes::Bytes;
use futures::StreamExt;
use nym_ip_packet_requests::{codec::MultiIpPacketCodec, v8::response::ControlResponse};
use tokio_util::codec::FramedRead;
use tracing::{error, info, warn};

use crate::ip_packet_client::current::response::{
    InfoLevel, IpPacketResponse, IpPacketResponseData,
};
use crate::ip_packet_client::helpers::check_ipr_message_version;

pub enum MixnetMessageOutcome {
    IpPackets(Vec<Bytes>),
    Disconnect,
}

pub struct IprListener {}

#[derive(Debug, thiserror::Error)]
pub enum IprListenerError {
    #[error(transparent)]
    IprClientError(#[from] crate::Error),
}

impl From<super::error::Error> for IprListenerError {
    fn from(err: super::error::Error) -> Self {
        match err {
            super::error::Error::SdkError(sdk_err) => IprListenerError::IprClientError(*sdk_err),
            other => IprListenerError::IprClientError(crate::Error::new_unsupported(format!(
                "IP packet error: {}",
                other
            ))),
        }
    }
}

impl IprListener {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse raw IPR response bytes into an outcome.
    pub async fn handle_response(
        &mut self,
        data: &[u8],
    ) -> Result<Option<MixnetMessageOutcome>, IprListenerError> {
        check_ipr_message_version(data)?;

        match IpPacketResponse::from_bytes(data) {
            Ok(response) => match response.data {
                IpPacketResponseData::Data(data_response) => {
                    let framed_reader = FramedRead::new(
                        data_response.ip_packet.as_ref(),
                        MultiIpPacketCodec::new(),
                    );
                    let responses: Vec<Bytes> = framed_reader
                        .filter_map(|res| async { res.ok().map(|packet| packet.into_bytes()) })
                        .collect()
                        .await;
                    return Ok(Some(MixnetMessageOutcome::IpPackets(responses)));
                }
                IpPacketResponseData::Control(control_response) => match *control_response {
                    ControlResponse::Connect(_) => {
                        info!("Received connect response when already connected - ignoring");
                    }
                    ControlResponse::Disconnect(_) => {
                        info!("Received disconnect response");
                        return Ok(Some(MixnetMessageOutcome::Disconnect));
                    }
                    ControlResponse::UnrequestedDisconnect(_) => {
                        info!("Received unrequested disconnect response, ignoring for now");
                    }
                    ControlResponse::Pong(_) => {
                        info!("Received pong response, ignoring for now");
                    }
                    ControlResponse::Health(_) => {
                        info!("Received health response, ignoring for now");
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
}

impl Default for IprListener {
    fn default() -> Self {
        Self::new()
    }
}
