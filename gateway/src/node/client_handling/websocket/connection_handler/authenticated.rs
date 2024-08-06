// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::{
    bandwidth::{Bandwidth, BandwidthError},
    websocket::{
        connection_handler::{
            ecash::error::EcashTicketError, ClientBandwidth, ClientDetails, FreshHandler,
        },
        message_receiver::{
            IsActive, IsActiveRequestReceiver, IsActiveResultSender, MixMessageReceiver,
        },
    },
    FREE_TESTNET_BANDWIDTH_VALUE,
};
use futures::{
    future::{FusedFuture, OptionFuture},
    FutureExt, StreamExt,
};
use nym_credentials::ecash::utils::{ecash_today, EcashTime};
use nym_credentials_interface::{ClientTicket, CredentialSpendingData};
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_requests::{
    types::{BinaryRequest, ServerResponse},
    ClientControlRequest, GatewayRequestsError, SimpleGatewayRequestsError,
};
use nym_gateway_storage::{error::StorageError, Storage};
use nym_sphinx::forwarding::packet::MixPacket;
use nym_task::TaskClient;
use nym_validator_client::coconut::EcashApiError;
use rand::{CryptoRng, Rng};
use si_scale::helpers::bibytes2;
use std::{process, time::Duration};
use thiserror::Error;
use time::{Date, OffsetDateTime};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};
use tracing::*;

#[derive(Debug, Error)]
pub enum RequestHandlingError {
    #[error("Internal gateway storage error")]
    StorageError(#[from] StorageError),

    #[error(
        "the database entry for bandwidth of the registered client {client_address} is missing!"
    )]
    MissingClientBandwidthEntry { client_address: String },

    #[error("Provided binary request was malformed - {0}")]
    InvalidBinaryRequest(#[from] GatewayRequestsError),

    #[error("Provided binary request was malformed - {0}")]
    InvalidTextRequest(<ClientControlRequest as TryFrom<String>>::Error),

    #[error("The received request is not valid in the current context: {additional_context}")]
    IllegalRequest { additional_context: String },

    #[error("credential has been rejected by the validators")]
    RejectedProposal,

    #[error(
        "the provided credential has an invalid spending date. got {got} but expected {expected}"
    )]
    InvalidCredentialSpendingDate { got: Date, expected: Date },

    #[error("the provided bandwidth credential has already been spent before at this gateway")]
    BandwidthCredentialAlreadySpent,

    #[error("This gateway is only accepting coconut credentials for bandwidth")]
    OnlyCoconutCredentials,

    #[error("Nyxd Error - {0}")]
    NyxdError(#[from] nym_validator_client::nyxd::error::NyxdError),

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

    #[error("insufficient bandwidth available to process the request. required: {required}B, available: {available}B")]
    OutOfBandwidth { required: i64, available: i64 },

    #[error(transparent)]
    EcashFailure(EcashTicketError),

    #[error(
        "the received payment contained more than a single ticket. that's currently not supported"
    )]
    MultipleTickets,
}

impl RequestHandlingError {
    fn into_error_message(self) -> Message {
        let server_response = match self {
            RequestHandlingError::OutOfBandwidth {
                required,
                available,
            } => ServerResponse::TypedError {
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

impl From<EcashTicketError> for RequestHandlingError {
    fn from(err: EcashTicketError) -> Self {
        // don't expose storage issue details to the user
        if let EcashTicketError::InternalStorageFailure { source } = err {
            RequestHandlingError::StorageError(source)
        } else {
            RequestHandlingError::EcashFailure(err)
        }
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
    client_bandwidth: ClientBandwidth,
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
    St: Storage + Clone + 'static,
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
    pub(crate) async fn upgrade(
        fresh: FreshHandler<R, S, St>,
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

        Ok(AuthenticatedHandler {
            inner: fresh,
            client,
            client_bandwidth: ClientBandwidth::new(bandwidth.into()),
            mix_receiver,
            is_active_request_receiver,
            is_active_ping_pending_reply: None,
        })
    }

    /// Explicitly removes handle from the global store.
    fn disconnect(self) {
        self.inner
            .active_clients_store
            .disconnect(self.client.address)
    }

    async fn expire_bandwidth(&mut self) -> Result<(), RequestHandlingError> {
        self.inner.expire_bandwidth(self.client.id).await?;
        self.client_bandwidth.bandwidth = Default::default();
        self.client_bandwidth.update_sync_data();
        Ok(())
    }

    /// Increases the amount of available bandwidth of the connected client by the specified value.
    ///
    /// # Arguments
    ///
    /// * `amount`: amount to increase the available bandwidth by.
    /// * `expiration` : the expiration date of that bandwidth
    async fn increase_bandwidth(
        &mut self,
        bandwidth: Bandwidth,
        expiration: OffsetDateTime,
    ) -> Result<(), RequestHandlingError> {
        self.client_bandwidth.bandwidth.bytes += bandwidth.value() as i64;
        self.client_bandwidth.bytes_delta_since_sync += bandwidth.value() as i64;
        self.client_bandwidth.bandwidth.expiration = expiration;

        // any increases to bandwidth should get flushed immediately
        // (we don't want to accidentally miss somebody claiming a gigabyte voucher)
        self.sync_expiration().await?;
        self.sync_bandwidth().await
    }

    /// Decreases the amount of available bandwidth of the connected client by the specified value.
    ///
    /// # Arguments
    ///
    /// * `amount`: amount to decrease the available bandwidth by.
    async fn consume_bandwidth(&mut self, amount: i64) -> Result<(), RequestHandlingError> {
        self.client_bandwidth.bandwidth.bytes -= amount;
        self.client_bandwidth.bytes_delta_since_sync -= amount;

        // since we're going to be operating on a fair use policy anyway, even if we crash and let extra few packets
        // through, that's completely fine
        if self
            .client_bandwidth
            .should_sync(self.inner.shared_state.bandwidth_cfg)
        {
            self.sync_bandwidth().await?;
        }

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

    async fn check_local_db_for_double_spending(
        &self,
        serial_number: &[u8],
    ) -> Result<(), RequestHandlingError> {
        trace!("checking local db for double spending...");

        let spent = self
            .inner
            .shared_state
            .storage
            .contains_ticket(serial_number)
            .await?;
        if spent {
            trace!("the credential has already been spent before at this gateway");
            return Err(RequestHandlingError::BandwidthCredentialAlreadySpent);
        }
        Ok(())
    }

    async fn check_bloomfilter(&self, serial_number: &Vec<u8>) -> Result<(), RequestHandlingError> {
        trace!("checking the bloomfilter...");

        let spent = self
            .inner
            .shared_state
            .ecash_verifier
            .check_double_spend(serial_number)
            .await;

        if spent {
            trace!("the credential has already been spent before at some gateway before (bloomfilter failure)");
            return Err(RequestHandlingError::BandwidthCredentialAlreadySpent);
        }
        Ok(())
    }

    fn check_credential_spending_date(
        &self,
        proposed: Date,
        today: Date,
    ) -> Result<(), RequestHandlingError> {
        trace!("checking ticket spending date...");

        if today != proposed {
            trace!("invalid credential spending date. received {proposed}");
            return Err(RequestHandlingError::InvalidCredentialSpendingDate {
                got: proposed,
                expected: today,
            });
        }
        Ok(())
    }

    async fn cryptographically_verify_ticket(
        &self,
        credential: &CredentialSpendingRequest,
    ) -> Result<(), RequestHandlingError> {
        trace!("attempting to perform ticket verification...");

        let aggregated_verification_key = self
            .inner
            .shared_state
            .ecash_verifier
            .verification_key(credential.data.epoch_id)
            .await?;

        self.inner
            .shared_state
            .ecash_verifier
            .check_payment(&credential.data, &aggregated_verification_key)
            .await?;
        Ok(())
    }

    fn async_verify_ticket(&self, ticket: CredentialSpendingData, ticket_id: i64) {
        let client_ticket = ClientTicket::new(ticket, ticket_id);

        self.inner
            .shared_state
            .ecash_verifier
            .async_verify(client_ticket);
    }

    async fn store_received_ticket(
        &self,
        ticket_data: &CredentialSpendingRequest,
        received_at: OffsetDateTime,
    ) -> Result<i64, RequestHandlingError> {
        trace!("storing received ticket");
        let ticket_id = self
            .inner
            .shared_state
            .storage
            .insert_received_ticket(
                self.client.id,
                received_at,
                ticket_data.encoded_serial_number(),
                ticket_data.to_bytes(),
            )
            .await?;
        Ok(ticket_id)
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
        let received_at = OffsetDateTime::now_utc();
        // TODO: change it into a span field instead once we move to tracing
        debug!(
            "handling e-cash bandwidth request from {}",
            self.client.address
        );

        let credential = ClientControlRequest::try_from_enc_ecash_credential(
            enc_credential,
            &self.client.shared_keys,
            iv,
        )?;
        let spend_date = ecash_today();

        // check if the credential hasn't been spent before
        let serial_number = credential.data.encoded_serial_number();

        if credential.data.payment.spend_value != 1 {
            return Err(RequestHandlingError::MultipleTickets);
        }

        self.check_credential_spending_date(credential.data.spend_date, spend_date.ecash_date())?;
        self.check_bloomfilter(&serial_number).await?;
        self.check_local_db_for_double_spending(&serial_number)
            .await?;

        // TODO: do we HAVE TO do it?
        self.cryptographically_verify_ticket(&credential).await?;

        let ticket_id = self.store_received_ticket(&credential, received_at).await?;
        self.async_verify_ticket(credential.data, ticket_id);

        // TODO: double storing?
        // self.store_spent_credential(serial_number_bs58).await?;

        let bandwidth = Bandwidth::ticket_amount(Default::default());

        self.increase_bandwidth(bandwidth, spend_date).await?;

        let available_total = self.client_bandwidth.bandwidth.bytes;

        Ok(ServerResponse::Bandwidth { available_total })
    }

    async fn handle_claim_testnet_bandwidth(
        &mut self,
    ) -> Result<ServerResponse, RequestHandlingError> {
        debug!("handling testnet bandwidth request");

        if self.inner.shared_state.only_coconut_credentials {
            return Err(RequestHandlingError::OnlyCoconutCredentials);
        }

        self.increase_bandwidth(FREE_TESTNET_BANDWIDTH_VALUE, ecash_today())
            .await?;
        let available_total = self.client_bandwidth.bandwidth.bytes;

        Ok(ServerResponse::Bandwidth { available_total })
    }

    async fn sync_expiration(&mut self) -> Result<(), RequestHandlingError> {
        self.inner
            .shared_state
            .storage
            .set_expiration(self.client.id, self.client_bandwidth.bandwidth.expiration)
            .await?;
        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    async fn sync_bandwidth(&mut self) -> Result<(), RequestHandlingError> {
        trace!("syncing client bandwidth with the underlying storage");
        let updated = self
            .inner
            .shared_state
            .storage
            .increase_bandwidth(self.client.id, self.client_bandwidth.bytes_delta_since_sync)
            .await?;

        trace!(updated);

        self.client_bandwidth.bandwidth.bytes = updated;

        self.client_bandwidth.update_sync_data();
        Ok(())
    }

    #[instrument(skip_all)]
    async fn try_use_bandwidth(
        &mut self,
        required_bandwidth: i64,
    ) -> Result<i64, RequestHandlingError> {
        if self.client_bandwidth.bandwidth.expired() {
            self.expire_bandwidth().await?;
        }
        let available_bandwidth = self.client_bandwidth.bandwidth.bytes;

        if available_bandwidth < required_bandwidth {
            return Err(RequestHandlingError::OutOfBandwidth {
                required: required_bandwidth,
                available: available_bandwidth,
            });
        }

        let available_bi2 = bibytes2(available_bandwidth as f64);
        let required_bi2 = bibytes2(required_bandwidth as f64);
        debug!(available = available_bi2, required = required_bi2);

        self.consume_bandwidth(required_bandwidth).await?;
        Ok(available_bandwidth)
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

        let remaining_bandwidth = self.try_use_bandwidth(required_bandwidth).await?;
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
        trace!("text request");
        match ClientControlRequest::try_from(raw_request) {
            Err(e) => RequestHandlingError::InvalidTextRequest(e).into_error_message(),
            Ok(request) => match request {
                ClientControlRequest::EcashCredential { enc_credential, iv } => self
                    .handle_ecash_bandwidth(enc_credential, iv)
                    .await
                    .into_ws_message(),
                ClientControlRequest::BandwidthCredential { .. } => {
                    RequestHandlingError::IllegalRequest {
                        additional_context: "coconut credential are not longer supported".into(),
                    }
                    .into_error_message()
                }
                ClientControlRequest::BandwidthCredentialV2 { .. } => {
                    RequestHandlingError::IllegalRequest {
                        additional_context: "coconut credential are not longer supported".into(),
                    }
                    .into_error_message()
                }
                ClientControlRequest::ClaimFreeTestnetBandwidth => self
                    .handle_claim_testnet_bandwidth()
                    .await
                    .into_ws_message(),
                other => RequestHandlingError::IllegalRequest {
                    additional_context: format!(
                        "received illegal message of type {} in an authenticated client",
                        other.name()
                    ),
                }
                .into_error_message(),
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
        // TODO: this should be added via tracing
        debug!(
            "handling request from {}",
            self.client.address.as_base58_string()
        );

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

        self.disconnect();
        trace!("The stream was closed!");
    }
}
