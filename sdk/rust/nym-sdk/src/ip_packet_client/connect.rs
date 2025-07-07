// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{sync::Arc, time::Duration};

pub use crate::mixnet::{
    InputMessage, MixnetClient, MixnetClientSender, MixnetMessageSender, Recipient,
    TransmissionLane,
};
use nym_gateway_directory::IpPacketRouterAddress;
use nym_ip_packet_requests::IpPair;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use crate::{
    ip_packet_client::current::{
        request::IpPacketRequest,
        response::{
            ConnectResponse, ConnectResponseReply, ControlResponse, IpPacketResponse,
            IpPacketResponseData,
        },
    },
    ip_packet_client::helpers::check_ipr_message_version,
};
use super::error::{Error, Result};


pub type SharedMixnetClient = Arc<tokio::sync::Mutex<Option<MixnetClient>>>;

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
    // As such, we drop the shared mixnet client once we're connected.
    mixnet_client: SharedMixnetClient,
    mixnet_sender: MixnetClientSender,
    connected: ConnectionState,
    cancel_token: CancellationToken,
}

impl IprClientConnect {
    pub async fn new(mixnet_client: SharedMixnetClient, cancel_token: CancellationToken) -> Self {
        let mixnet_sender = mixnet_client.lock().await.as_ref().unwrap().split_sender();
        Self {
            mixnet_client,
            mixnet_sender,
            connected: ConnectionState::Disconnected,
            cancel_token,
        }
    }

    pub async fn connect(
        &mut self,
        ip_packet_router_address: IpPacketRouterAddress,
    ) -> Result<IpPair> {
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

    async fn connect_inner(
        &mut self,
        ip_packet_router_address: IpPacketRouterAddress,
    ) -> Result<IpPair> {
        let request_id = self.send_connect_request(ip_packet_router_address).await?;

        debug!("Waiting for reply...");
        self.listen_for_connect_response(request_id).await
    }

    async fn send_connect_request(
        &mut self,
        ip_packet_router_address: IpPacketRouterAddress,
    ) -> Result<u64> {
        let (request, request_id) = IpPacketRequest::new_connect_request(None);

        // We use 20 surbs for the connect request because typically the IPR is configured to have
        // a min threshold of 10 surbs that it reserves for itself to request additional surbs.
        let surbs = 20;
        self.mixnet_sender
            .send(create_input_message(
                Recipient::from(ip_packet_router_address),
                request,
                surbs,
            ))
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

    async fn listen_for_connect_response(&self, request_id: u64) -> Result<IpPair> {
        // Connecting is basically synchronous from the perspective of the mixnet client, so it's safe
        // to just grab ahold of the mutex and keep it until we get the response.
        let mut mixnet_client_handle = self.mixnet_client.lock().await;
        let mixnet_client = mixnet_client_handle.as_mut().unwrap();

        let timeout = sleep(IPR_CONNECT_TIMEOUT);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    error!("Cancelled while waiting for reply to connect request");
                    return Err(Error::Cancelled);
                },
                _ = &mut timeout => {
                    error!("Timed out waiting for reply to connect request");
                    return Err(Error::TimeoutWaitingForConnectResponse);
                },
                msgs = mixnet_client.wait_for_messages() => match msgs {
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
) -> InputMessage {
    InputMessage::new_anonymous(
        recipient,
        request.to_bytes().unwrap(),
        surbs,
        TransmissionLane::General,
        None,
    )
}
