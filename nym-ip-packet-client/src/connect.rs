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

use crate::{
    current::{
        request::IpPacketRequest,
        response::{
            ConnectResponse, ConnectResponseReply, ControlResponse, IpPacketResponse,
            IpPacketResponseData,
        },
    },
    error::{Error, Result},
    helpers::check_ipr_message_version,
};

const IPR_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
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

    pub async fn connect(&mut self, ip_packet_router_address: Recipient) -> Result<IpPair> {
        if self.connected != ConnectionState::Disconnected {
            return Err(Error::AlreadyConnected);
        }

        tracing::info!("Connecting to exit gateway");
        self.connected = ConnectionState::Connecting;
        match self.connect_inner(ip_packet_router_address).await {
            Ok(ips) => {
                debug!("Successfully connected to the ip-packet-router");
                self.connected = ConnectionState::Connected;
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

        debug!("Waiting for reply...");
        self.listen_for_connect_response(request_id).await
    }

    async fn send_connect_request(&self, ip_packet_router_address: Recipient) -> Result<u64> {
        let (request, request_id) = IpPacketRequest::new_connect_request(None);

        // We use 20 surbs for the connect request because typically the IPR is configured to have
        // a min threshold of 10 surbs that it reserves for itself to request additional surbs.
        let surbs = 20;
        self.mixnet_client
            .send(create_input_message(
                ip_packet_router_address,
                request,
                surbs,
            )?)
            .await
            .map_err(|err| Error::SdkError(Box::new(err)))?;

        Ok(request_id)
    }

    async fn handle_connect_response(&self, response: ConnectResponse) -> Result<IpPair> {
        debug!("Handling dynamic connect response");
        match response.reply {
            ConnectResponseReply::Success(r) => Ok(r.ips),
            ConnectResponseReply::Failure(reason) => Err(Error::ConnectRequestDenied { reason }),
        }
    }

    async fn handle_ip_packet_router_response(&self, response: IpPacketResponse) -> Result<IpPair> {
        let control_response = match response.data {
            IpPacketResponseData::Control(control_response) => control_response,
            _ => {
                error!("Received non-control response while waiting for connect response");
                return Err(Error::UnexpectedConnectResponse);
            }
        };

        match *control_response {
            ControlResponse::Connect(resp) => self.handle_connect_response(resp).await,
            response => {
                error!("Unexpected response: {response:?}");
                Err(Error::UnexpectedConnectResponse)
            }
        }
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
                                tracing::info!("Mixnet message version mismatch: {err}");
                                continue;
                            }

                            // Then we deserialize the message
                            tracing::debug!("IprClient: got message while waiting for connect response");
                            let Ok(response) = IpPacketResponse::from_reconstructed_message(&msg) else {
                                // This is ok, it's likely just one of our self-pings
                                tracing::debug!("Failed to deserialize mixnet message");
                                continue;
                            };

                            if response.id() == Some(request_id) {
                                tracing::debug!("Got response with matching id");
                                return self.handle_ip_packet_router_response(response).await;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn create_input_message(
    recipient: Recipient,
    request: IpPacketRequest,
    surbs: u32,
) -> Result<InputMessage> {
    Ok(InputMessage::new_anonymous(
        recipient,
        request.to_bytes()?,
        surbs,
        TransmissionLane::General,
        None,
    ))
}
