// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::bandwidth::{Bandwidth, BandwidthError};
use crate::node::client_handling::websocket::connection_handler::ClientBandwidth;
use crate::node::{
    client_handling::{
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
use futures::{
    future::{FusedFuture, OptionFuture},
    FutureExt, StreamExt,
};
use log::*;
use nym_credentials::coconut::bandwidth::CredentialType;
use nym_credentials_interface::{to_coconut, CoconutBase58, CoconutError, CoconutParameters};
use nym_gateway_requests::models::OldCredentialSpendingRequest;
use nym_gateway_requests::{
    iv::{IVConversionError, IV},
    types::{BinaryRequest, ServerResponse},
    ClientControlRequest, GatewayRequestsError,
};
use nym_sphinx::forwarding::packet::MixPacket;
use nym_task::TaskClient;
use nym_validator_client::coconut::CoconutApiError;
use rand::{CryptoRng, Rng};
use std::{process, time::Duration};
use thiserror::Error;
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};

#[derive(Debug, Error)]
pub enum RequestHandlingError {
    #[error("Internal gateway storage error")]
    StorageError(#[from] StorageError),

    #[error(
        "the database entry for bandwidth of the registered client {client_address} is missing!"
    )]
    MissingClientBandwidthEntry { client_address: String },

    #[error("Provided bandwidth IV is malformed - {0}")]
    MalformedIV(#[from] IVConversionError),

    #[error("Provided binary request was malformed - {0}")]
    InvalidBinaryRequest(#[from] GatewayRequestsError),

    #[error("Provided binary request was malformed - {0}")]
    InvalidTextRequest(<ClientControlRequest as TryFrom<String>>::Error),

    #[error("The received request is not valid in the current context: {additional_context}")]
    IllegalRequest { additional_context: String },

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

    #[error("Compact ecash error - {0}")]
    CompactEcashError(#[from] nym_credentials_interface::CompactEcashError),

    #[error("coconut failure: {0}")]
    CoconutError(#[from] CoconutError),

    #[error("coconut api query failure: {0}")]
    CoconutApiError(#[from] CoconutApiError),

    #[error("Credential error - {0}")]
    CredentialError(#[from] nym_credentials::error::Error),

    #[error("Internal error")]
    InternalError,

    #[error("failed to recover bandwidth value: {0}")]
    BandwidthRecoveryFailure(#[from] BandwidthError),

    #[error("the provided credential did not contain a valid type attribute")]
    InvalidTypeAttribute,

    #[error("insufficient bandwidth available to process the request. required: {required}B, available: {available}B")]
    OutOfBandwidth { required: i64, available: i64 },

    #[error("the provided credential did not have a bandwidth attribute")]
    MissingBandwidthAttribute,

    #[error("attempted to claim a bandwidth voucher for an account using a free pass (it expires on {expiration})")]
    BandwidthVoucherForFreePassAccount { expiration: OffsetDateTime },

    #[error("attempted to claim another free pass for the account while another free pass is still active (it expires on {expiration})")]
    PreexistingFreePass { expiration: OffsetDateTime },

    #[error("the DKG contract is unavailable")]
    UnavailableDkgContract,
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
            .storage
            .get_available_bandwidth(client.address)
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

    async fn expire_freepass(&mut self) -> Result<(), RequestHandlingError> {
        self.client_bandwidth.bandwidth = Default::default();
        self.client_bandwidth.update_flush_data();
        Ok(self.inner.expire_freepass(self.client.address).await?)
    }

    /// Increases the amount of available bandwidth of the connected client by the specified value.
    ///
    /// # Arguments
    ///
    /// * `amount`: amount to increase the available bandwidth by.
    async fn increase_bandwidth(
        &mut self,
        bandwidth: Bandwidth,
    ) -> Result<(), RequestHandlingError> {
        self.client_bandwidth.bandwidth.bytes += bandwidth.value() as i64;

        // any increases to bandwidth should get flushed immediately
        // (we don't want to accidentally miss somebody claiming a gigabyte voucher)
        self.flush_bandwidth().await
    }

    async fn set_freepass_expiration(
        &mut self,
        expiration: OffsetDateTime,
    ) -> Result<(), RequestHandlingError> {
        self.client_bandwidth.bandwidth.freepass_expiration = Some(expiration);
        self.inner
            .storage
            .set_freepass_expiration(self.client.address, expiration)
            .await?;
        self.client_bandwidth.update_flush_data();
        Ok(())
    }

    /// Decreases the amount of available bandwidth of the connected client by the specified value.
    ///
    /// # Arguments
    ///
    /// * `amount`: amount to decrease the available bandwidth by.
    async fn consume_bandwidth(&mut self, amount: i64) -> Result<(), RequestHandlingError> {
        self.client_bandwidth.bandwidth.bytes -= amount;

        // since we're going to be operating on a fair use policy anyway, even if we crash and let extra few packets
        // through, that's completely fine
        if self
            .client_bandwidth
            .should_flush(self.inner.shared_state.bandwidth_cfg)
        {
            self.flush_bandwidth().await?;
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

    async fn handle_bandwidth_request(
        &mut self,
        credential: OldCredentialSpendingRequest,
    ) -> Result<ServerResponse, RequestHandlingError> {
        // check if the credential hasn't been spent before
        let blinded_serial_number_bs58 = credential.data.blinded_serial_number().to_bs58();
        trace!("processing credential {}", blinded_serial_number_bs58);

        // if we already have had received a free pass (that's not expired, don't accept any additional bandwidth)
        if self.client_bandwidth.bandwidth.freepass_expired() {
            // the free pass we used before has expired -> reset our state and handle the request as normal
            self.expire_freepass().await?;
        } else if let Some(expiration) = self.client_bandwidth.bandwidth.freepass_expiration {
            // the free pass is still valid -> return error
            return match credential.data.typ {
                CredentialType::Voucher => {
                    Err(RequestHandlingError::BandwidthVoucherForFreePassAccount { expiration })
                }
                CredentialType::FreePass => {
                    Err(RequestHandlingError::PreexistingFreePass { expiration })
                }
            };
        }

        let already_spent = self
            .inner
            .storage
            .contains_credential(blinded_serial_number_bs58.clone())
            .await?;
        if already_spent {
            trace!("the credential has already been spent before");
            return Err(RequestHandlingError::BandwidthCredentialAlreadySpent);
        }

        trace!(
            "attempting to obtain aggregate verification key for epoch {}",
            credential.data.epoch_id
        );

        if !credential.data.validate_type_attribute() {
            trace!("mismatch in the type attribute");
            return Err(RequestHandlingError::InvalidTypeAttribute);
        }

        let Some(bandwidth_attribute) = credential.data.get_bandwidth_attribute() else {
            trace!("missing bandwidth attribute");
            return Err(RequestHandlingError::MissingBandwidthAttribute);
        };

        // this will extract token amounts out of bandwidth vouchers and validate expiry of free passes
        let (raw_bandwidth, freepass_expiration) =
            Bandwidth::parse_raw_bandwidth(bandwidth_attribute, credential.data.typ)?;

        let bandwidth = Bandwidth::new(raw_bandwidth)?;

        trace!("embedded bandwidth: {bandwidth:?}");

        // locally verify the credential
        {
            let aggregated_verification_key = self
                .inner
                .shared_state
                .coconut_verifier
                .verification_key(credential.data.epoch_id)
                .await?;

            //4 is legacy from old coconut credential
            // safety: the unwrap is fine here as 4 is non-zero
            #[allow(clippy::unwrap_used)]
            let params = CoconutParameters::new(4).unwrap();
            if !credential
                .data
                .verify(&params, &to_coconut(&aggregated_verification_key)?)
            {
                trace!("the credential did not verify correctly");
                return Err(RequestHandlingError::InvalidBandwidthCredential(
                    String::from("local credential verification has failed"),
                ));
            }
        }

        match credential.data.typ {
            CredentialType::TicketBook => {
                //Not really a ticketbook, but it's for backwards compatibility
                trace!("the credential is a bandwidth voucher. attempting to release the funds");
                let api_clients = self
                    .inner
                    .shared_state
                    .ecash_verifier
                    .api_clients(credential.data.epoch_id)
                    .await?;
                self.inner
                    .shared_state
                    .ecash_verifier
                    .release_old_bandwidth_voucher_funds(&api_clients, credential)
                    .await?;
            }
            CredentialType::FreePass => {
                // no need to do anything special here, we already extracted the bandwidth amount and checked expiry
                info!("received a free pass credential");
            }
        }

        // technically this is not atomic, i.e. checking for the spending and then marking as spent,
        // but because we have the `UNIQUE` constraint on the database table
        // if somebody attempts to spend the same credential in another, parallel request,
        // one of them will fail
        //
        // mark the credential as spent
        // TODO: technically this should be done under a storage transaction so that if we experience any
        // failures later on, it'd get reverted
        trace!("storing serial number information");
        self.inner
            .storage
            .insert_spent_credential(
                blinded_serial_number_bs58,
                freepass_expiration.is_some(),
                self.client.address,
            )
            .await?;

        trace!("increasing client bandwidth");
        self.increase_bandwidth(bandwidth).await?;
        // set free pass expiration
        if let Some(expiration) = freepass_expiration {
            self.set_freepass_expiration(expiration).await?;
        }

        let available_total = self.client_bandwidth.bandwidth.bytes;

        Ok(ServerResponse::Bandwidth { available_total })
    }

    async fn handle_bandwidth_v1(
        &mut self,
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    ) -> Result<ServerResponse, RequestHandlingError> {
        debug!("handling v1 bandwidth request");

        let iv = IV::try_from_bytes(&iv)?;
        let credential = ClientControlRequest::try_from_enc_coconut_bandwidth_credential_v1(
            enc_credential,
            &self.client.shared_keys,
            iv,
        )?;

        self.handle_bandwidth_request(credential.try_into()?).await
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
        debug!("handling ecash bandwidth request");

        let iv = IV::try_from_bytes(&iv)?;
        let credential = ClientControlRequest::try_from_enc_ecash_credential(
            enc_credential,
            &self.client.shared_keys,
            iv,
        )?;

        // check if the credential hasn't been spent before
        let serial_number_bs58 = credential.data.serial_number_b58();
        trace!("processing credential {}", serial_number_bs58);

        let already_spent = self
            .inner
            .storage
            .contains_credential(serial_number_bs58.clone())
            .await?;
        if already_spent {
            trace!("the credential has already been spent before");
            return Err(RequestHandlingError::BandwidthCredentialAlreadySpent);
        }

        trace!(
            "attempting to obtain aggregate verification key for epoch {}",
            credential.data.epoch_id
        );

        let aggregated_verification_key = self
            .inner
            .ecash_verifier
            .verification_key(credential.data.epoch_id)
            .await?;

        self.inner
            .ecash_verifier
            .check_payment(&credential.data, &aggregated_verification_key)
            .await?;

        let was_freepass = match credential.data.typ {
            CredentialType::TicketBook => {
                trace!("the credential is a bandwidth voucher");
                let api_clients = self
                    .inner
                    .ecash_verifier
                    .api_clients(credential.data.epoch_id)
                    .await?;

                if self.inner.offline_credential_verification {
                    self.inner
                        .ecash_verifier
                        .post_credential(&api_clients, credential.clone())
                        .await?;

                    self.inner
                        .storage
                        .insert_credential(credential.clone())
                        .await?;
                } else {
                    self.inner
                        .ecash_verifier
                        .release_bandwidth_voucher_funds(&api_clients, &credential)
                        .await?;
                }
                false
            }
            CredentialType::FreePass => {
                // no need to do anything special here, we already extracted the bandwidth amount and checked expiry
                info!("received a free pass credential");

                true
            }
        };

        trace!("storing serial number information");
        self.inner
            .storage
            .insert_spent_credential(serial_number_bs58, was_freepass, self.client.address)
            .await?;

        let bandwidth = Bandwidth::get_for_type(credential.data.typ);

        self.increase_bandwidth(bandwidth).await?;
        let available_total = self.get_available_bandwidth().await?;

        Ok(ServerResponse::Bandwidth { available_total })
    }

    /// Tries to handle the received bandwidth request by checking correctness of the received data
    /// and if successful, increases client's bandwidth by an appropriate amount.
    ///
    /// # Arguments
    ///
    /// * `enc_credential`: raw encrypted bandwidth credential to verify.
    /// * `iv`: fresh iv used for the credential.
    async fn handle_bandwidth_v2(
        &mut self,
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    ) -> Result<ServerResponse, RequestHandlingError> {
        debug!("handling v2 bandwidth request");

        let iv = IV::try_from_bytes(&iv)?;
        let credential = ClientControlRequest::try_from_enc_coconut_bandwidth_credential_v2(
            enc_credential,
            &self.client.shared_keys,
            iv,
        )?;

        self.handle_bandwidth_request(credential).await
    }

    async fn handle_claim_testnet_bandwidth(
        &mut self,
    ) -> Result<ServerResponse, RequestHandlingError> {
        debug!("handling testnet bandwidth request");

        if self.inner.shared_state.only_coconut_credentials {
            return Err(RequestHandlingError::OnlyCoconutCredentials);
        }

        self.increase_bandwidth(FREE_TESTNET_BANDWIDTH_VALUE)
            .await?;
        let available_total = self.client_bandwidth.bandwidth.bytes;

        Ok(ServerResponse::Bandwidth { available_total })
    }

    async fn flush_bandwidth(&mut self) -> Result<(), RequestHandlingError> {
        trace!("flushing client bandwidth to the underlying storage");
        self.inner
            .storage
            .set_bandwidth(self.client.address, self.client_bandwidth.bandwidth.bytes)
            .await?;
        self.client_bandwidth.update_flush_data();
        Ok(())
    }

    async fn try_use_bandwidth(
        &mut self,
        required_bandwidth: i64,
    ) -> Result<i64, RequestHandlingError> {
        if self.client_bandwidth.bandwidth.freepass_expired() {
            self.expire_freepass().await?;
        }
        let available_bandwidth = self.client_bandwidth.bandwidth.bytes;

        if available_bandwidth < required_bandwidth {
            return Err(RequestHandlingError::OutOfBandwidth {
                required: required_bandwidth,
                available: available_bandwidth,
            });
        }

        self.consume_bandwidth(required_bandwidth).await?;
        Ok(self.client_bandwidth.bandwidth.bytes)
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
                ClientControlRequest::BandwidthCredential { enc_credential, iv } => self
                    .handle_bandwidth_v1(enc_credential, iv)
                    .await
                    .into_ws_message(),
                ClientControlRequest::BandwidthCredentialV2 { enc_credential, iv } => self
                    .handle_bandwidth_v2(enc_credential, iv)
                    .await
                    .into_ws_message(),
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
