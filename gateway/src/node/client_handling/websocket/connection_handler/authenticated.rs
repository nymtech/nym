// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::websocket::connection_handler::{ClientDetails, FreshHandler};
use crate::node::client_handling::websocket::message_receiver::MixMessageReceiver;
use crate::node::storage::error::StorageError;
use futures::StreamExt;
use gateway_requests::iv::IVConversionError;
use gateway_requests::types::{BinaryRequest, ServerResponse};
use gateway_requests::{ClientControlRequest, GatewayRequestsError};
use log::*;
use nymsphinx::forwarding::packet::MixPacket;
use rand::{CryptoRng, Rng};
use std::convert::TryFrom;
use std::process;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::node::client_handling::bandwidth::Bandwidth;
use gateway_requests::iv::IV;

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

    #[error("Provided bandwidth credential did not verify correctly")]
    InvalidBandwidthCredential,

    #[cfg(not(feature = "coconut"))]
    #[error("Ethereum web3 error")]
    Web3Error(#[from] web3::Error),

    #[cfg(not(feature = "coconut"))]
    #[error("Ethereum ABI error")]
    EthAbiError(#[from] web3::ethabi::Error),

    #[cfg(not(feature = "coconut"))]
    #[error("Ethereum contract error")]
    EthContractError(#[from] web3::contract::Error),

    #[cfg(not(feature = "coconut"))]
    #[error("Nymd Error - {0}")]
    NymdError(#[from] validator_client::nymd::error::NymdError),

    #[cfg(feature = "coconut")]
    #[error("Provided coconut bandwidth credential did not have expected structure - {0}")]
    CoconutBandwidthCredentialError(#[from] credentials::error::Error),
}

impl RequestHandlingError {
    fn into_error_message(self) -> Message {
        ServerResponse::new_error(self.to_string()).into()
    }
}

pub(crate) struct AuthenticatedHandler<R, S> {
    inner: FreshHandler<R, S>,
    client: ClientDetails,
    mix_receiver: MixMessageReceiver,
}

// explicitly remove handle from the global store upon being dropped
impl<R, S> Drop for AuthenticatedHandler<R, S> {
    fn drop(&mut self) {
        self.inner
            .active_clients_store
            .disconnect(self.client.address)
    }
}

impl<R, S> AuthenticatedHandler<R, S>
where
    // TODO: those trait bounds here don't really make sense....
    R: Rng + CryptoRng,
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
        fresh: FreshHandler<R, S>,
        client: ClientDetails,
        mix_receiver: MixMessageReceiver,
    ) -> Self {
        AuthenticatedHandler {
            inner: fresh,
            client,
            mix_receiver,
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
            error!("We failed to forward requested mix packet - {}. Presumably our mix forwarder has crashed. We cannot continue.", err);
            process::exit(1);
        }
    }

    #[cfg(feature = "coconut")]
    /// Tries to handle the received bandwidth request by checking correctness of the received data
    /// and if successful, increases client's bandwidth by an appropriate amount.
    ///
    /// # Arguments
    ///
    /// * `enc_credential`: raw encrypted bandwidth credential to verify.
    /// * `iv`: fresh iv used for the credential.
    async fn handle_coconut_bandwidth(
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

        if !credential.verify(&self.inner.aggregated_verification_key) {
            return Err(RequestHandlingError::InvalidBandwidthCredential);
        }

        let bandwidth = Bandwidth::try_from(credential)?;
        let bandwidth_value = bandwidth.value();

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

    #[cfg(not(feature = "coconut"))]
    /// Tries to handle the received bandwidth request by checking correctness of the received data
    /// and if successful, increases client's bandwidth by an appropriate amount.
    ///
    /// # Arguments
    ///
    /// * `enc_credential`: raw encrypted bandwidth credential to verify.
    /// * `iv`: fresh iv used for the credential.
    async fn handle_token_bandwidth(
        &mut self,
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    ) -> Result<ServerResponse, RequestHandlingError> {
        let iv = IV::try_from_bytes(&iv)?;
        let credential = ClientControlRequest::try_from_enc_token_bandwidth_credential(
            enc_credential,
            &self.client.shared_keys,
            iv,
        )?;

        debug!("Received bandwidth increase request. Verifying signature");
        if !credential.verify_signature() {
            return Err(RequestHandlingError::InvalidBandwidthCredential);
        }
        debug!("Verifying Ethereum for token burn...");
        self.inner
            .erc20_bridge
            .verify_eth_events(credential.verification_key())
            .await?;
        debug!("Verifying Cosmos for double spending...");
        self.inner
            .erc20_bridge
            .verify_double_spending(&credential)
            .await?;

        let bandwidth = Bandwidth::from(credential);
        let bandwidth_value = bandwidth.value();

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
        debug!("Increased bandwidth for client: {:?}", self.client.address);

        Ok(ServerResponse::Bandwidth { available_total })
    }

    async fn handle_bandwidth(
        &mut self,
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    ) -> Result<ServerResponse, RequestHandlingError> {
        #[cfg(feature = "coconut")]
        return self.handle_coconut_bandwidth(enc_credential, iv).await;
        #[cfg(not(feature = "coconut"))]
        return self.handle_token_bandwidth(enc_credential, iv).await;
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
        let consumed_bandwidth = mix_packet.sphinx_packet().len() as i64;

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
            Err(e) => RequestHandlingError::InvalidBinaryRequest(e).into_error_message(),
            Ok(request) => match request {
                // currently only a single type exists
                BinaryRequest::ForwardSphinx(mix_packet) => {
                    match self.handle_forward_sphinx(mix_packet).await {
                        Ok(response) => response.into(),
                        Err(err) => err.into_error_message(),
                    }
                }
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
                ClientControlRequest::BandwidthCredential { enc_credential, iv } => {
                    match self.handle_bandwidth(enc_credential, iv).await {
                        Ok(response) => response.into(),
                        Err(err) => err.into_error_message(),
                    }
                }
                _ => RequestHandlingError::IllegalRequest.into_error_message(),
            },
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
            _ => None,
        }
    }

    /// Simultaneously listens for incoming client requests, which realistically should only be
    /// binary requests to forward sphinx packets or increase bandwidth
    /// and for sphinx packets received from the mix network that should be sent back to the client.
    pub(crate) async fn listen_for_requests(mut self)
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        trace!("Started listening for ALL incoming requests...");

        loop {
            tokio::select! {
                socket_msg = self.inner.read_websocket_message() => {
                    let socket_msg = match socket_msg {
                        None => break,
                        Some(Ok(socket_msg)) => socket_msg,
                        Some(Err(err)) => {
                            error!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                            break;
                        }
                    };

                    if socket_msg.is_close() {
                        break;
                    }

                    if let Some(response) = self.handle_request(socket_msg).await {
                        if let Err(err) = self.inner.send_websocket_message(response).await {
                            warn!(
                                "Failed to send message over websocket: {}. Assuming the connection is dead.",
                                err
                            );
                            break;
                        }
                    }
                },
                mix_messages = self.mix_receiver.next() => {
                    let mix_messages = mix_messages.expect("sender was unexpectedly closed! this shouldn't have ever happened!");
                    if let Err(e) = self.inner.push_packets_to_client(self.client.shared_keys, mix_messages).await {
                        warn!("failed to send the unwrapped sphinx packets back to the client - {:?}, assuming the connection is dead", e);
                        break;
                    }
                }
            }
        }

        self.disconnect();
        trace!("The stream was closed!");
    }
}
