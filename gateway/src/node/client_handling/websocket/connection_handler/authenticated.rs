// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::{
    bandwidth::BandwidthError,
    websocket::{
        connection_handler::{ClientDetails, FreshHandler},
        message_receiver::{
            IsActive, IsActiveRequestReceiver, IsActiveResultSender, MixMessageReceiver,
        },
    },
};
use futures::{
    future::{FusedFuture, OptionFuture},
    FutureExt, StreamExt,
};
use nym_credential_verification::CredentialVerifier;
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, ClientBandwidth,
};
use nym_gateway_requests::{
    types::{BinaryRequest, ServerResponse},
    ClientControlRequest, ClientRequest, GatewayRequestsError, SensitiveServerResponse,
    SimpleGatewayRequestsError,
};
use nym_gateway_storage::error::GatewayStorageError;
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use nym_gateway_storage::traits::SharedKeyGatewayStorage;
use nym_node_metrics::events::MetricsEvent;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_statistics_common::{gateways::GatewaySessionEvent, types::SessionType};
use nym_task::TaskClient;
use nym_validator_client::coconut::EcashApiError;
use rand::{random, CryptoRng, Rng};
use std::{process, time::Duration};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};
use tracing::*;

#[derive(Debug, Error)]
pub enum RequestHandlingError {
    #[error("Internal gateway storage error")]
    StorageError(#[from] GatewayStorageError),

    #[error(
        "the database entry for bandwidth of the registered client {client_address} is missing!"
    )]
    MissingClientBandwidthEntry { client_address: String },

    #[error("received a binary request of an unknown type")]
    UnknownBinaryRequest,

    #[error("received a text request of an unknown type")]
    UnknownTextRequest,

    #[error("received an encrypted text request of an unknown type")]
    UnknownEncryptedTextRequest,

    #[error("Provided binary request was malformed - {0}")]
    InvalidBinaryRequest(#[from] GatewayRequestsError),

    #[error("failed to decrypt provided text request")]
    InvalidEncryptedTextRequest,

    #[error("Provided binary request was malformed - {0}")]
    InvalidTextRequest(<ClientControlRequest as TryFrom<String>>::Error),

    #[error("The received request is not valid in the current context: {additional_context}")]
    IllegalRequest { additional_context: String },

    #[error("credential has been rejected by the validators")]
    RejectedProposal,

    #[error("Validator API error - {0}")]
    APIError(#[from] nym_validator_client::ValidatorClientError),

    #[error("There was a problem with the proposal id: {reason}")]
    ProposalIdError { reason: String },

    #[error("compact ecash error: {0}")]
    CompactEcashError(#[from] nym_credentials_interface::CompactEcashError),

    #[error("coconut api query failure: {0}")]
    CoconutApiError(#[from] EcashApiError),

    #[error("Credential error - {0}")]
    CredentialError(#[from] nym_credentials::error::Error),

    #[error("Internal error")]
    InternalError,

    #[error("failed to recover bandwidth value: {0}")]
    BandwidthRecoveryFailure(#[from] BandwidthError),

    #[error("{0}")]
    CredentialVerification(#[from] nym_credential_verification::Error),
}

impl RequestHandlingError {
    fn into_error_message(self) -> Message {
        let server_response = match self {
            RequestHandlingError::CredentialVerification(
                nym_credential_verification::Error::OutOfBandwidth {
                    required,
                    available,
                },
            ) => ServerResponse::TypedError {
                error: SimpleGatewayRequestsError::OutOfBandwidth {
                    required,
                    available,
                },
            },
            other => ServerResponse::new_error(other.to_string()),
        };
        server_response.into()
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

impl IntoWSMessage for ServerResponse {
    fn into_ws_message(self) -> Message {
        self.into()
    }
}

pub(crate) struct AuthenticatedHandler<R, S> {
    inner: FreshHandler<R, S>,
    bandwidth_storage_manager: BandwidthStorageManager,
    client: ClientDetails,
    mix_receiver: MixMessageReceiver,
    // Occasionally the handler is requested to ping the connected client for confirm that it's
    // active, such as when a duplicate connection is detected. This hashmap stores the oneshot
    // senders that are used to return the result of the ping to the handler requesting the ping.
    is_active_request_receiver: IsActiveRequestReceiver,
    is_active_ping_pending_reply: Option<(u64, IsActiveResultSender)>,
}

// explicitly remove handle from the global store upon being dropped
impl<R, S> Drop for AuthenticatedHandler<R, S> {
    fn drop(&mut self) {
        self.disconnect_client()
    }
}

impl<R, S> AuthenticatedHandler<R, S> {
    pub(crate) fn inner(&self) -> &FreshHandler<R, S> {
        &self.inner
    }

    /// Upgrades `FreshHandler` into the Authenticated variant implying the client is now authenticated
    /// and thus allowed to perform more actions with the gateway, such as redeeming bandwidth or
    /// sending sphinx packets.
    ///
    /// # Arguments
    ///
    /// * `fresh`: fresh, unauthenticated, connection handler.
    /// * `client`: details (i.e. address and shared keys) of the registered client
    /// * `mix_receiver`: channel used for receiving messages from the mixnet destined for this client.
    pub(crate) async fn upgrade(
        fresh: FreshHandler<R, S>,
        client: ClientDetails,
        mix_receiver: MixMessageReceiver,
        is_active_request_receiver: IsActiveRequestReceiver,
    ) -> Result<Self, RequestHandlingError> {
        // note: the `upgrade` function can only be called after registering or authenticating the client,
        // meaning the appropriate database rows must have been created
        // so in theory we could just unwrap the value here, but since we're returning a Result anyway,
        // we might as well return a failure response instead
        let bandwidth = fresh
            .shared_state
            .storage
            .get_available_bandwidth(client.id)
            .await?
            .ok_or(RequestHandlingError::MissingClientBandwidthEntry {
                client_address: client.address.as_base58_string(),
            })?;

        let handler = AuthenticatedHandler {
            bandwidth_storage_manager: BandwidthStorageManager::new(
                Box::new(fresh.shared_state.storage.clone()),
                ClientBandwidth::new(bandwidth.into()),
                client.id,
                fresh.shared_state.cfg.bandwidth,
                fresh.shared_state.cfg.enforce_zk_nym,
            ),
            inner: fresh,
            client,
            mix_receiver,
            is_active_request_receiver,
            is_active_ping_pending_reply: None,
        };
        handler.send_metrics(GatewaySessionEvent::new_session_start(
            handler.client.address,
        ));

        Ok(handler)
    }

    fn disconnect_client(&mut self) {
        self.inner
            .shared_state
            .active_clients_store
            .disconnect(self.client.address);
        self.send_metrics(GatewaySessionEvent::new_session_stop(self.client.address));
    }

    fn send_metrics(&self, event: impl Into<MetricsEvent>) {
        self.inner.send_metrics(event)
    }

    /// Forwards the received mix packet from the client into the mix network.
    ///
    /// # Arguments
    ///
    /// * `mix_packet`: packet received from the client that should get forwarded into the network.
    fn forward_packet(&self, mix_packet: MixPacket) {
        if let Err(err) = self
            .inner
            .shared_state
            .outbound_mix_sender
            .forward_packet(mix_packet)
        {
            error!("We failed to forward requested mix packet - {err}. Presumably our mix forwarder has crashed. We cannot continue.");
            process::exit(1);
        }
    }

    /// Tries to handle the received bandwidth request by checking correctness of the received data
    /// and if successful, increases client's bandwidth by an appropriate amount.
    ///
    /// # Arguments
    ///
    /// * `enc_credential`: raw encrypted credential to verify.
    /// * `iv`: fresh iv used for the credential.
    async fn handle_ecash_bandwidth(
        &mut self,
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    ) -> Result<ServerResponse, RequestHandlingError> {
        debug!("handling e-cash bandwidth request");

        let credential = ClientControlRequest::try_from_enc_ecash_credential(
            enc_credential,
            &self.client.shared_keys,
            iv,
        )?;

        let mut verifier = CredentialVerifier::new(
            credential,
            self.inner.shared_state.ecash_verifier.clone(),
            self.bandwidth_storage_manager.clone(),
        );

        let available_total = verifier
            .verify()
            .await
            .inspect_err(|verification_failure| debug!("{verification_failure}"))?;
        trace!("available total bandwidth: {available_total}");

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
    #[instrument(skip_all)]
    async fn handle_forward_sphinx(
        &mut self,
        mix_packet: MixPacket,
    ) -> Result<ServerResponse, RequestHandlingError> {
        let required_bandwidth = mix_packet.packet().len() as i64;

        let remaining_bandwidth = self
            .bandwidth_storage_manager
            .try_use_bandwidth(required_bandwidth)
            .await?;
        self.forward_packet(mix_packet);

        Ok(ServerResponse::Send {
            remaining_bandwidth,
        })
    }

    /// Attempts to handle a binary data frame websocket message.
    ///
    /// # Arguments
    ///
    /// * `bin_msg`: raw message to handle.
    async fn handle_binary(&mut self, bin_msg: Vec<u8>) -> Message {
        trace!("binary request");
        // this function decrypts the request and checks the MAC
        match BinaryRequest::try_from_encrypted_tagged_bytes(bin_msg, &self.client.shared_keys) {
            Err(e) => {
                error!("{e}");
                RequestHandlingError::InvalidBinaryRequest(e).into_error_message()
            }
            Ok(request) => match request {
                // currently only a single type exists
                BinaryRequest::ForwardSphinx { packet }
                | BinaryRequest::ForwardSphinxV2 { packet } => {
                    self.handle_forward_sphinx(packet).await.into_ws_message()
                }
                _ => RequestHandlingError::UnknownBinaryRequest.into_error_message(),
            },
        }
    }

    async fn handle_forget_me(
        &mut self,
        client: bool,
        _stats: bool,
    ) -> Result<ServerResponse, RequestHandlingError> {
        if client {
            self.inner()
                .shared_state()
                .storage()
                .handle_forget_me(self.client.address)
                .await?;
        }
        Ok(SensitiveServerResponse::ForgetMeAck {}.encrypt(&self.client.shared_keys)?)
    }

    async fn handle_remember_me(
        &self,
        session_type: SessionType,
    ) -> Result<ServerResponse, RequestHandlingError> {
        self.send_metrics(GatewaySessionEvent::new_session_remember(
            session_type,
            self.client.address,
        ));
        Ok(SensitiveServerResponse::RememberMeAck {}.encrypt(&self.client.shared_keys)?)
    }

    async fn handle_key_upgrade(
        &mut self,
        hkdf_salt: Vec<u8>,
        client_key_digest: Vec<u8>,
    ) -> Result<ServerResponse, RequestHandlingError> {
        if !self.client.shared_keys.is_legacy() {
            return Ok(ServerResponse::new_error(
                "the connection is already using an aes256-gcm-siv key",
            ));
        }
        let legacy_key = self.client.shared_keys.unwrap_legacy();
        let Some(upgraded_key) = legacy_key.upgrade_verify(&hkdf_salt, &client_key_digest) else {
            return Ok(ServerResponse::new_error(
                "failed to derive matching aes256-gcm-siv key",
            ));
        };

        let updated_key = upgraded_key.into();
        self.inner
            .shared_state
            .storage
            .insert_shared_keys(self.client.address, &updated_key)
            .await?;

        // swap the in-memory key
        self.client.shared_keys = updated_key;
        Ok(SensitiveServerResponse::KeyUpgradeAck {}.encrypt(&self.client.shared_keys)?)
    }

    async fn handle_encrypted_text_request(
        &mut self,
        ciphertext: Vec<u8>,
        nonce: Vec<u8>,
    ) -> Result<ServerResponse, RequestHandlingError> {
        let Ok(req) = ClientRequest::decrypt(&ciphertext, &nonce, &self.client.shared_keys) else {
            return Err(RequestHandlingError::InvalidEncryptedTextRequest);
        };

        match req {
            ClientRequest::UpgradeKey {
                hkdf_salt,
                derived_key_digest,
            } => self.handle_key_upgrade(hkdf_salt, derived_key_digest).await,
            ClientRequest::ForgetMe { client, stats } => self.handle_forget_me(client, stats).await,
            ClientRequest::RememberMe { session_type } => {
                self.handle_remember_me(session_type).await
            }
            _ => Err(RequestHandlingError::UnknownEncryptedTextRequest),
        }
    }

    /// Attempts to handle a text data frame websocket message.
    ///
    /// Currently the bandwidth credential request is the only one we can receive after authentication.
    ///
    /// # Arguments
    ///
    /// * `raw_request`: raw message to handle.
    async fn handle_text(&mut self, raw_request: String) -> Message
    where
        R: Rng + CryptoRng,
    {
        trace!("text request");

        let request = match ClientControlRequest::try_from(raw_request) {
            Ok(req) => {
                debug!("received request of type {}", req.name());
                req
            }
            Err(err) => {
                debug!("request was malformed: {err}");
                return RequestHandlingError::InvalidTextRequest(err).into_error_message();
            }
        };

        match request {
            ClientControlRequest::EncryptedRequest { ciphertext, nonce } => {
                self.handle_encrypted_text_request(ciphertext, nonce).await
            }
            ClientControlRequest::EcashCredential { enc_credential, iv } => {
                self.handle_ecash_bandwidth(enc_credential, iv).await
            }
            ClientControlRequest::BandwidthCredential { .. } => {
                Err(RequestHandlingError::IllegalRequest {
                    additional_context: "coconut credential are not longer supported".into(),
                })
            }
            ClientControlRequest::BandwidthCredentialV2 { .. } => {
                Err(RequestHandlingError::IllegalRequest {
                    additional_context: "coconut credential are not longer supported".into(),
                })
            }
            ClientControlRequest::ClaimFreeTestnetBandwidth => self
                .bandwidth_storage_manager
                .handle_claim_testnet_bandwidth()
                .await
                .map_err(|e| e.into()),
            ClientControlRequest::SupportedProtocol { .. } => {
                Ok(self.inner.handle_supported_protocol_request())
            }
            other @ ClientControlRequest::Authenticate { .. } => {
                Err(RequestHandlingError::IllegalRequest {
                    additional_context: format!(
                        "received illegal message of type {} in an authenticated client",
                        other.name()
                    ),
                })
            }
            other @ ClientControlRequest::RegisterHandshakeInitRequest { .. } => {
                Err(RequestHandlingError::IllegalRequest {
                    additional_context: format!(
                        "received illegal message of type {} in an authenticated client",
                        other.name()
                    ),
                })
            }
            _ => Err(RequestHandlingError::UnknownTextRequest),
        }
        .inspect(|res| debug!(response = ?res, "success"))
        .inspect_err(|err| debug!(error = %err, "failure"))
        .into_ws_message()
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
    #[instrument(level = "debug", skip_all,
        fields(
            client = %self.client.address.as_base58_string()
        )
    )]
    async fn handle_request(&mut self, raw_request: Message) -> Option<Message>
    where
        R: Rng + CryptoRng,
    {
        trace!("new request");

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
        let tag: u64 = random();
        debug!("got request to ping our connection: {tag}");
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
        R: Rng + CryptoRng,
        S: AsyncRead + AsyncWrite + Unpin,
    {
        trace!("Started listening for ALL incoming requests...");

        // Ping timeout future used to check if the client responded to our ping request
        let mut ping_timeout: OptionFuture<_> = None.into();

        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = shutdown.recv() => {
                    trace!("client_handling::AuthenticatedHandler: received shutdown");
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
                            debug!("failed to obtain message from websocket stream! stopping connection handler: {err}");
                            break;
                        }
                    };

                    if socket_msg.is_close() {
                        break;
                    }

                    if let Some(response) = self.handle_request(socket_msg).await {
                        if let Err(err) = self.inner.send_websocket_message(response).await {
                            debug!(
                                "Failed to send message over websocket: {err}. Assuming the connection is dead.",
                            );
                            break;
                        }
                    }
                },
                mix_messages = self.mix_receiver.next() => {
                    let mix_messages = match mix_messages {
                        None => {
                            debug!("mix receiver was closed! Assuming the connection is dead.");
                            break;
                        }
                        Some(mix_messages) => mix_messages,
                    };
                    if let Err(err) = self.inner.push_packets_to_client(&self.client.shared_keys, mix_messages).await {
                        debug!("failed to send the unwrapped sphinx packets back to the client - {err}, assuming the connection is dead");
                        break;
                    }
                }
            }
        }

        trace!("The stream was closed!");
    }
}
