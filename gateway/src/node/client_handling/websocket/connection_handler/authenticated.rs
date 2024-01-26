// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::{
    future::{FusedFuture, OptionFuture},
    FutureExt, StreamExt,
};
use log::*;
use nym_gateway_requests::{
    iv::{IVConversionError, IV},
    types::{BinaryRequest, ServerResponse},
    ClientControlRequest, GatewayRequestsError,
};
use nym_sphinx::forwarding::packet::MixPacket;
use nym_task::TaskClient;
use nym_validator_client::coconut::CoconutApiError;
use rand::{CryptoRng, Rng};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};

use std::cmp::max;
use std::{convert::TryFrom, process, time::Duration};

use crate::node::client_handling::websocket::connection_handler::coconut::BANDWIDTH_PER_CREDENTIAL;
use crate::node::{
    client_handling::{
        bandwidth::Bandwidth,
        websocket::{
            connection_handler::{ClientDetails, FreshHandler},
            message_receiver::{
                IsActive, IsActiveRequestReceiver, IsActiveResultSender, MixMessageReceiver,
            },
        },
        FREE_TESTNET_BANDWIDTH_VALUE,
    },
    storage::{error::StorageError, Storage},
};

#[derive(Debug, Error)]
pub(crate) enum RequestHandlingError {
    #[error("Internal gateway storage error")]
    StorageError(#[from] StorageError),

    #[error("Provided bandwidth IV is malformed - {0}")]
    MalformedIV(#[from] IVConversionError),

    #[error("Provided binary request was malformed - {0}")]
    InvalidBinaryRequest(#[from] GatewayRequestsError),

    #[error("Provided binary request was malformed - {0}")]
    InvalidTextRequest(<ClientControlRequest as TryFrom<String>>::Error),

    #[error("The received request is not valid in the current context")]
    IllegalRequest,

    #[error("Provided bandwidth credential asks for more bandwidth than it is supported to add at once (credential value: {0}, supported: {}). Try to split it before attempting again", i64::MAX)]
    UnsupportedBandwidthValue(u64),

    #[error("Provided bandwidth credential did not verify correctly on {0}")]
    InvalidBandwidthCredential(String),

    #[error("the provided bandwidth credential has already been spent before at this gateway")]
    BandwidthCredentialAlreadySpent,

    #[error("This gateway is only accepting coconut credentials for bandwidth")]
    OnlyCoconutCredentials,

    #[error("Nyxd Error - {0}")]
    NyxdError(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("Validator API error - {0}")]
    APIError(#[from] nym_validator_client::ValidatorClientError),

    #[error("Not enough nym API endpoints provided. Needed {needed}, received {received}")]
    NotEnoughNymAPIs { received: usize, needed: usize },

    #[error("There was a problem with the proposal id: {reason}")]
    ProposalIdError { reason: String },

    #[error("Coconut interface error - {0}")]
    CoconutInterfaceError(#[from] nym_coconut_interface::error::CoconutInterfaceError),

    #[error("coconut api query failure: {0}")]
    CoconutApiError(#[from] CoconutApiError),

    #[error("Credential error - {0}")]
    CredentialError(#[from] nym_credentials::error::Error),
}

impl RequestHandlingError {
    fn into_error_message(self) -> Message {
        ServerResponse::new_error(self.to_string()).into()
    }
}

/// Helper trait that allows converting result of handling client request into a websocket message
// Note: I couldn't have implemented a normal "From" trait as both `Message` and `Result` are foreign types
trait IntoWSMessage {
    fn into_ws_message(self) -> Message;
}

impl IntoWSMessage for Result<ServerResponse, RequestHandlingError> {
    fn into_ws_message(self) -> Message {
        match self {
            Ok(response) => response.into(),
            Err(err) => err.into_error_message(),
        }
    }
}

pub(crate) struct AuthenticatedHandler<R, S, St> {
    inner: FreshHandler<R, S, St>,
    client: ClientDetails,
    mix_receiver: MixMessageReceiver,
    // Occasionally the handler is requested to ping the connected client for confirm that it's
    // active, such as when a duplicate connection is detected. This hashmap stores the oneshot
    // senders that are used to return the result of the ping to the handler requesting the ping.
    is_active_request_receiver: IsActiveRequestReceiver,
    is_active_ping_pending_reply: Option<(u64, IsActiveResultSender)>,
}

// explicitly remove handle from the global store upon being dropped
impl<R, S, St> Drop for AuthenticatedHandler<R, S, St> {
    fn drop(&mut self) {
        self.inner
            .active_clients_store
            .disconnect(self.client.address)
    }
}

impl<R, S, St> AuthenticatedHandler<R, S, St>
where
    // TODO: those trait bounds here don't really make sense....
    R: Rng + CryptoRng,
    St: Storage,
{
    /// Upgrades `FreshHandler` into the Authenticated variant implying the client is now authenticated
    /// and thus allowed to perform more actions with the gateway, such as redeeming bandwidth or
    /// sending sphinx packets.
    ///
    /// # Arguments
    ///
    /// * `fresh`: fresh, unauthenticated, connection handler.
    /// * `client`: details (i.e. address and shared keys) of the registered client
    /// * `mix_receiver`: channel used for receiving messages from the mixnet destined for this client.
    pub(crate) fn upgrade(
        fresh: FreshHandler<R, S, St>,
        client: ClientDetails,
        mix_receiver: MixMessageReceiver,
        is_active_request_receiver: IsActiveRequestReceiver,
    ) -> Self {
        AuthenticatedHandler {
            inner: fresh,
            client,
            mix_receiver,
            is_active_request_receiver,
            is_active_ping_pending_reply: None,
        }
    }

    /// Explicitly removes handle from the global store.
    fn disconnect(self) {
        self.inner
            .active_clients_store
            .disconnect(self.client.address)
    }

    /// Checks the amount of bandwidth available for the connected client.
    async fn get_available_bandwidth(&self) -> Result<i64, RequestHandlingError> {
        let bandwidth = self
            .inner
            .storage
            .get_available_bandwidth(self.client.address)
            .await?
            .unwrap_or_default();
        Ok(bandwidth)
    }

    /// Increases the amount of available bandwidth of the connected client by the specified value.
    ///
    /// # Arguments
    ///
    /// * `amount`: amount to increase the available bandwidth by.
    async fn increase_bandwidth(&self, amount: i64) -> Result<(), RequestHandlingError> {
        self.inner
            .storage
            .increase_bandwidth(self.client.address, amount)
            .await?;
        Ok(())
    }

    /// Decreases the amount of available bandwidth of the connected client by the specified value.
    ///
    /// # Arguments
    ///
    /// * `amount`: amount to decrease the available bandwidth by.
    async fn consume_bandwidth(&self, amount: i64) -> Result<(), RequestHandlingError> {
        self.inner
            .storage
            .consume_bandwidth(self.client.address, amount)
            .await?;
        Ok(())
    }

    /// Forwards the received mix packet from the client into the mix network.
    ///
    /// # Arguments
    ///
    /// * `mix_packet`: packet received from the client that should get forwarded into the network.
    fn forward_packet(&self, mix_packet: MixPacket) {
        if let Err(err) = self.inner.outbound_mix_sender.unbounded_send(mix_packet) {
            error!("We failed to forward requested mix packet - {err}. Presumably our mix forwarder has crashed. We cannot continue.");
            process::exit(1);
        }
    }

    /// Tries to handle the received bandwidth request by checking correctness of the received data
    /// and if successful, increases client's bandwidth by an appropriate amount.
    ///
    /// # Arguments
    ///
    /// * `enc_credential`: raw encrypted bandwidth credential to verify.
    /// * `iv`: fresh iv used for the credential.
    async fn handle_bandwidth(
        &mut self,
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    ) -> Result<ServerResponse, RequestHandlingError> {
        let iv = IV::try_from_bytes(&iv)?;
        let credential = ClientControlRequest::try_from_enc_coconut_bandwidth_credential(
            enc_credential,
            &self.client.shared_keys,
            iv,
        )?;

        // check if the credential hasn't been spent before
        let already_spent = self
            .inner
            .storage
            .contains_credential(credential.blinded_serial_number())
            .await?;
        if already_spent {
            return Err(RequestHandlingError::BandwidthCredentialAlreadySpent);
        }

        // locally verify the credential
        let aggregated_verification_key = self
            .inner
            .coconut_verifier
            .verification_key(*credential.epoch_id())
            .await?;

        if !credential.verify(&aggregated_verification_key) {
            return Err(RequestHandlingError::InvalidBandwidthCredential(
                String::from("credential failed to verify on gateway"),
            ));
        }

        // technically this is not atomic, i.e. checking for the spending and then marking as spent,
        // but because we have the `UNIQUE` constraint on the database table
        // if somebody attempts to spend the same credential in another, parallel request,
        // one of them will fail
        //
        // mark the credential as spent
        // TODO: technically this should be done under a storage transaction so that if we experience any
        // failures later on, it'd get reverted
        self.inner
            .storage
            .insert_spent_credential(*credential.blinded_serial_number(), self.client.address)
            .await?;

        // OLD CODE FOR RELEASING FUNDS
        // let api_clients = self
        //     .inner
        //     .coconut_verifier
        //     .api_clients(*credential.epoch_id())
        //     .await?;
        //
        // self.inner
        //     .coconut_verifier
        //     .release_funds(&api_clients, &credential)
        //     .await?;

        let bandwidth = Bandwidth::from(credential);

        // if somebody decided to use a credential with bunch of tokens in it, sure, grant them that bandwidth
        // otherwise use the default value
        let bandwidth_value = max(bandwidth.value(), BANDWIDTH_PER_CREDENTIAL);

        if bandwidth_value > i64::MAX as u64 {
            // note that this would have represented more than 1 exabyte,
            // which is like 125,000 worth of hard drives so I don't think we have
            // to worry about it for now...
            warn!("Somehow we received bandwidth value higher than 9223372036854775807. We don't really want to deal with this now");
            return Err(RequestHandlingError::UnsupportedBandwidthValue(
                bandwidth_value,
            ));
        }

        self.increase_bandwidth(bandwidth_value as i64).await?;
        let available_total = self.get_available_bandwidth().await?;

        Ok(ServerResponse::Bandwidth { available_total })
    }

    async fn handle_claim_testnet_bandwidth(
        &mut self,
    ) -> Result<ServerResponse, RequestHandlingError> {
        if self.inner.only_coconut_credentials {
            return Err(RequestHandlingError::OnlyCoconutCredentials);
        }

        self.increase_bandwidth(FREE_TESTNET_BANDWIDTH_VALUE)
            .await?;
        let available_total = self.get_available_bandwidth().await?;

        Ok(ServerResponse::Bandwidth { available_total })
    }

    /// Tries to handle request to forward sphinx packet into the network. The request can only succeed
    /// if the client has enough available bandwidth.
    ///
    /// Upon forwarding, client's bandwidth is decreased by the size of the forwarded packet.
    ///
    /// # Arguments
    ///
    /// * `mix_packet`: packet received from the client that should get forwarded into the network.
    async fn handle_forward_sphinx(
        &self,
        mix_packet: MixPacket,
    ) -> Result<ServerResponse, RequestHandlingError> {
        let consumed_bandwidth = mix_packet.packet().len() as i64;

        let available_bandwidth = self.get_available_bandwidth().await?;

        if available_bandwidth < consumed_bandwidth {
            return Ok(ServerResponse::new_error(
                "Insufficient bandwidth available",
            ));
        }

        self.consume_bandwidth(consumed_bandwidth).await?;
        self.forward_packet(mix_packet);

        Ok(ServerResponse::Send {
            remaining_bandwidth: available_bandwidth - consumed_bandwidth,
        })
    }

    /// Attempts to handle a binary data frame websocket message.
    ///
    /// # Arguments
    ///
    /// * `bin_msg`: raw message to handle.
    async fn handle_binary(&self, bin_msg: Vec<u8>) -> Message {
        // this function decrypts the request and checks the MAC
        match BinaryRequest::try_from_encrypted_tagged_bytes(bin_msg, &self.client.shared_keys) {
            Err(e) => {
                error!("{e}");
                RequestHandlingError::InvalidBinaryRequest(e).into_error_message()
            }
            Ok(request) => match request {
                // currently only a single type exists
                BinaryRequest::ForwardSphinx(mix_packet) => self
                    .handle_forward_sphinx(mix_packet)
                    .await
                    .into_ws_message(),
            },
        }
    }

    /// Attempts to handle a text data frame websocket message.
    ///
    /// Currently the bandwidth credential request is the only one we can receive after authentication.
    ///
    /// # Arguments
    ///
    /// * `raw_request`: raw message to handle.
    async fn handle_text(&mut self, raw_request: String) -> Message {
        match ClientControlRequest::try_from(raw_request) {
            Err(e) => RequestHandlingError::InvalidTextRequest(e).into_error_message(),
            Ok(request) => match request {
                ClientControlRequest::BandwidthCredential { enc_credential, iv } => self
                    .handle_bandwidth(enc_credential, iv)
                    .await
                    .into_ws_message(),
                ClientControlRequest::ClaimFreeTestnetBandwidth => self
                    .handle_claim_testnet_bandwidth()
                    .await
                    .into_ws_message(),
                _ => RequestHandlingError::IllegalRequest.into_error_message(),
            },
        }
    }

    /// Handles pong message received from the client.
    /// If the client is still active, the handler that requested the ping will receive a reply.
    async fn handle_pong(&mut self, msg: Vec<u8>) {
        if let Ok(msg) = msg.try_into() {
            let msg = u64::from_be_bytes(msg);
            trace!("Received pong from client: {msg}");
            if let Some((tag, _)) = &self.is_active_ping_pending_reply {
                if tag == &msg {
                    debug!("Reporting back to the handler that the client is still active");
                    // safety:
                    // the unwrap here is fine as we can only enter this if branch if `self.is_active_ping_pending_reply`
                    // was a `Some`
                    #[allow(clippy::unwrap_used)]
                    let tx = self.is_active_ping_pending_reply.take().unwrap().1;
                    if let Err(err) = tx.send(IsActive::Active) {
                        warn!("Failed to send pong reply back to the requesting handler: {err:?}");
                    }
                } else {
                    warn!("Received pong reply from the client with unexpected tag: {msg}",);
                }
            }
        } else {
            warn!("the received pong message was not a valid u64")
        }
    }

    /// Attempts to handle websocket message received from the connected client.
    ///
    /// # Arguments
    ///
    /// * `raw_request`: raw received websocket message.
    async fn handle_request(&mut self, raw_request: Message) -> Option<Message> {
        // apparently tungstenite auto-handles ping/pong/close messages so for now let's ignore
        // them and let's test that claim. If that's not the case, just copy code from
        // desktop nym-client websocket as I've manually handled everything there
        match raw_request {
            Message::Binary(bin_msg) => Some(self.handle_binary(bin_msg).await),
            Message::Text(text_msg) => Some(self.handle_text(text_msg).await),
            Message::Pong(msg) => {
                self.handle_pong(msg).await;
                None
            }
            _ => None,
        }
    }

    /// Send a ping to the connected client and return a tag identifying the ping.
    async fn send_ping(&mut self) -> Result<u64, WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let tag: u64 = rand::thread_rng().gen();
        debug!("Got request to ping our connection: {}", tag);
        self.inner
            .send_websocket_message(Message::Ping(tag.to_be_bytes().to_vec()))
            .await?;
        Ok(tag)
    }

    /// Handles the ping timeout by responding back to the handler that requested the ping.
    async fn handle_ping_timeout(&mut self) {
        debug!("Ping timeout expired!");
        if let Some((_tag, reply_tx)) = self.is_active_ping_pending_reply.take() {
            if let Err(err) = reply_tx.send(IsActive::NotActive) {
                warn!("Failed to respond back to the handler requesting the ping: {err:?}");
            }
        }
    }

    async fn handle_is_active_request(
        &mut self,
        reply_tx: IsActiveResultSender,
    ) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        if self.is_active_ping_pending_reply.is_some() {
            warn!("Received request to ping the client, but a ping is already in progress!");
            if let Err(err) = reply_tx.send(IsActive::BusyPinging) {
                warn!("Failed to respond back to the handler requesting the ping: {err:?}");
            }
            return Ok(());
        }

        match self.send_ping().await {
            Ok(tag) => {
                self.is_active_ping_pending_reply = Some((tag, reply_tx));
                Ok(())
            }
            Err(err) => {
                warn!("Failed to send ping to client: {err}. Assuming the connection is dead.");
                Err(err)
            }
        }
    }

    /// Simultaneously listens for incoming client requests, which realistically should only be
    /// binary requests to forward sphinx packets or increase bandwidth
    /// and for sphinx packets received from the mix network that should be sent back to the client.
    pub(crate) async fn listen_for_requests(mut self, mut shutdown: TaskClient)
    where
        S: AsyncRead + AsyncWrite + Unpin,
        St: Storage,
    {
        trace!("Started listening for ALL incoming requests...");

        // Ping timeout future used to check if the client responded to our ping request
        let mut ping_timeout: OptionFuture<_> = None.into();

        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = shutdown.recv() => {
                    log::trace!("client_handling::AuthenticatedHandler: received shutdown");
                },
                // Received a request to ping the client to check if it's still active
                tx = self.is_active_request_receiver.next() => {
                    match tx {
                        None => break,
                        Some(reply_tx) => {
                            if self.handle_is_active_request(reply_tx).await.is_err() {
                                break;
                            }
                            // NOTE: fuse here due to .is_terminated() check below
                            ping_timeout = Some(Box::pin(tokio::time::sleep(Duration::from_millis(1000)).fuse())).into();
                        }
                    };
                },
                // The ping timeout expired, meaning the client didn't respond to our ping request
                _ = &mut ping_timeout, if !ping_timeout.is_terminated() => {
                   ping_timeout = None.into();
                   self.handle_ping_timeout().await;
                },
                socket_msg = self.inner.read_websocket_message() => {
                    let socket_msg = match socket_msg {
                        None => break,
                        Some(Ok(socket_msg)) => socket_msg,
                        Some(Err(err)) => {
                            error!("failed to obtain message from websocket stream! stopping connection handler: {err}");
                            break;
                        }
                    };

                    if socket_msg.is_close() {
                        break;
                    }

                    if let Some(response) = self.handle_request(socket_msg).await {
                        if let Err(err) = self.inner.send_websocket_message(response).await {
                            warn!(
                                "Failed to send message over websocket: {err}. Assuming the connection is dead.",
                            );
                            break;
                        }
                    }
                },
                mix_messages = self.mix_receiver.next() => {
                    let mix_messages = match mix_messages {
                        None => {
                            warn!("mix receiver was closed! Assuming the connection is dead.");
                            break;
                        }
                        Some(mix_messages) => mix_messages,
                    };
                    if let Err(err) = self.inner.push_packets_to_client(&self.client.shared_keys, mix_messages).await {
                        warn!("failed to send the unwrapped sphinx packets back to the client - {err}, assuming the connection is dead");
                        break;
                    }
                }
            }
        }

        self.disconnect();
        trace!("The stream was closed!");
    }
}
