// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use nym_ip_packet_requests::IpPair;
use nym_sdk::mixnet::{
    InputMessage, MixnetClient, MixnetMessageSender, Recipient, TransmissionLane,
};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use nym_ip_packet_requests::response_helpers::{self, IprResponseError};

use crate::{
    current::{self, response::IpPacketResponse},
    error::{Error, Result},
    helpers::check_ipr_message_version,
};

const IPR_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    Connecting {
        stream_id: u64,
    },
    Connected {
        stream_id: u64,
    },
    #[allow(unused)]
    Disconnecting,
}

pub struct IprClientConnect {
    // During connection we need the mixnet client, but once connected we expect to setup a channel
    // from the main mixnet listener at the top-level.
    mixnet_client: MixnetClient,
    connected: ConnectionState,
    cancel_token: CancellationToken,
}

impl IprClientConnect {
    pub fn new(mixnet_client: MixnetClient, cancel_token: CancellationToken) -> Self {
        Self {
            mixnet_client,
            connected: ConnectionState::Disconnected,
            cancel_token,
        }
    }

    pub fn into_mixnet_client(self) -> MixnetClient {
        self.mixnet_client
    }

    pub fn stream_id(&self) -> Option<u64> {
        match self.connected {
            ConnectionState::Connecting { stream_id }
            | ConnectionState::Connected { stream_id } => Some(stream_id),
            ConnectionState::Disconnected | ConnectionState::Disconnecting => None,
        }
    }

    pub async fn connect(&mut self, ip_packet_router_address: Recipient) -> Result<IpPair> {
        if self.connected != ConnectionState::Disconnected {
            return Err(Error::AlreadyConnected);
        }

        tracing::info!("Connecting to exit gateway");
        match self.connect_inner(ip_packet_router_address).await {
            Ok(ips) => {
                debug!("Successfully connected to the ip-packet-router");
                let Some(stream_id) = self.stream_id() else {
                    return Err(Error::UnexpectedConnectResponse);
                };
                self.connected = ConnectionState::Connected { stream_id };
                Ok(ips)
            }
            Err(err) => {
                error!("Failed to connect to the ip-packet-router: {:?}", err);
                self.connected = ConnectionState::Disconnected;
                Err(err)
            }
        }
    }

    async fn connect_inner(&mut self, ip_packet_router_address: Recipient) -> Result<IpPair> {
        let request_id = self.send_connect_request(ip_packet_router_address).await?;
        self.connected = ConnectionState::Connecting {
            stream_id: request_id,
        };

        debug!("Waiting for reply...");
        self.listen_for_connect_response(request_id).await
    }

    async fn send_connect_request(&mut self, ip_packet_router_address: Recipient) -> Result<u64> {
        let (request, request_id) = current::new_connect_request(None);
        tracing::info!(
            request_id = request_id,
            protocol_version = request.protocol.version,
            current_version = crate::current::VERSION,
            "Sending IPR connect request"
        );
        if let Ok(bytes) = request.to_bytes() {
            let prefix = bytes.get(0..2).unwrap_or(&bytes);
            let prefix_hex = format!("{:02x?}", prefix);
            tracing::info!(request_id = request_id, prefix = %prefix_hex, "IPR connect bytes prefix");
        }

        // We use 20 surbs for the connect request because typically the IPR is configured to have
        // a min threshold of 10 surbs that it reserves for itself to request additional surbs.
        let surbs = 20;
        let request_bytes = request.to_bytes()?;
        let framed_bytes = maybe_wrap_stream_frame(request_id, 0, request_bytes);
        self.mixnet_client
            .send(create_input_message(
                ip_packet_router_address,
                framed_bytes,
                surbs,
            )?)
            .await
            .map_err(|err| Error::SdkError(Box::new(err)))?;

        Ok(request_id)
    }

    async fn listen_for_connect_response(&mut self, request_id: u64) -> Result<IpPair> {
        // Connecting is basically synchronous from the perspective of the mixnet client, so it's safe
        // to just grab ahold of the mutex and keep it until we get the response.

        let timeout = sleep(IPR_CONNECT_TIMEOUT);
        tokio::pin!(timeout);

        let mixnet_cancel_token = self.mixnet_client.cancellation_token();

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    error!("Cancelled while waiting for reply to connect request");
                    return Err(Error::Cancelled);
                },

                _ = mixnet_cancel_token.cancelled() => {
                    error!("Mixnet client stopped while waiting for reply to connect request");
                    return Err(Error::Cancelled);
                },
                _ = &mut timeout => {
                    error!("Timed out waiting for reply to connect request");
                    return Err(Error::TimeoutWaitingForConnectResponse);
                },
                msgs = self.mixnet_client.wait_for_messages() => match msgs {
                    None => {
                        return Err(Error::NoMixnetMessagesReceived);
                    }
                    Some(msgs) => {
                        for msg in msgs {
                            // Confirm that the version is correct
                            if let Err(err) = check_ipr_message_version(&msg) {
                                let raw: &[u8] = msg.message.as_ref();
                                tracing::warn!(
                                    first_byte = raw.first().copied(),
                                    expected = crate::current::VERSION,
                                    len = raw.len(),
                                    "Mixnet message version mismatch: {err}"
                                );
                                continue;
                            }

                            // Then we deserialize the message
                            tracing::debug!("IprClient: got message while waiting for connect response");
                            let Ok(response) = ipr_response_from_reconstructed_message(&msg) else {
                                // This is ok, it's likely just one of our self-pings
                                tracing::debug!("Failed to deserialize mixnet message");
                                continue;
                            };

                            if response.id() == Some(request_id) {
                                tracing::debug!("Got response with matching id");
                                // Replaces local handle_ip_packet_router_response() + handle_connect_response()
                                return response_helpers::parse_connect_response(response)
                                    .map_err(|e| match e {
                                        IprResponseError::ConnectDenied(reason) => {
                                            Error::ConnectRequestDenied { reason }
                                        }
                                        _ => Error::UnexpectedConnectResponse,
                                    });
                            }
                        }
                    }
                }
            }
        }
    }
}

fn maybe_wrap_stream_frame(stream_id: u64, sequence_num: u32, payload: Vec<u8>) -> Vec<u8> {
    if !crate::lp_stream::current_requires_sphinx_stream_transport() {
        return payload;
    }

    crate::lp_stream::encode_stream_frame(stream_id, sequence_num, payload)
}

fn ipr_response_from_reconstructed_message(
    msg: &nym_sdk::mixnet::ReconstructedMessage,
) -> std::result::Result<IpPacketResponse, bincode::Error> {
    let payload = crate::lp_stream::unwrap_ipr_payload_from_reconstructed(msg);
    IpPacketResponse::from_bytes(payload)
}

fn create_input_message(recipient: Recipient, bytes: Vec<u8>, surbs: u32) -> Result<InputMessage> {
    Ok(InputMessage::new_anonymous(
        recipient,
        bytes,
        surbs,
        TransmissionLane::General,
        None,
    ))
}
