// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::lp_client::helpers::{convert_forward_data, try_convert_forward_response};
use crate::{LpClientError, LpRegistrationClient};
use nym_crypto::asymmetric::ed25519;
use nym_lp::message::ForwardPacketData;
use nym_lp::state_machine::{LpAction, LpInput};
use nym_lp_transport::traits::LpTransport;
use std::io;
use std::net::SocketAddr;

/// Attempt to treat the inner client as a LP connection
pub struct NestedConnection<'a, S> {
    /// Remote Ed25519 public key
    pub(crate) exit_identity: ed25519::PublicKey,

    /// Exit gateway's LP address (e.g., "2.2.2.2:41264")
    pub(crate) exit_address: SocketAddr,

    pub(crate) outer_client: &'a mut LpRegistrationClient<S>,
}

impl<'a, S> NestedConnection<'a, S> {
    async fn send_serialised_packet(&mut self, packet_data: &[u8]) -> Result<(), LpClientError>
    where
        S: LpTransport + Unpin,
    {
        let forward_packet_data =
            ForwardPacketData::new(self.exit_identity, self.exit_address, packet_data.to_vec());

        let target_address = self.exit_address;

        tracing::debug!(
            "Sending ForwardPacket to {target_address} ({} inner bytes, persistent connection)",
            forward_packet_data.inner_packet_bytes.len()
        );

        // 1. Serialize the ForwardPacketData
        let input = convert_forward_data(forward_packet_data)?;

        // 2. Encrypt and prepare packet via state machine
        let state_machine = self.outer_client.state_machine_mut()?;

        let action = state_machine
            .process_input(input)
            .ok_or_else(|| LpClientError::transport("State machine returned no action"))?
            .map_err(|e| {
                LpClientError::Transport(format!("Failed to encrypt ForwardPacket: {e}"))
            })?;

        let forward_packet = match action {
            LpAction::SendPacket(packet) => packet,
            other => {
                return Err(LpClientError::Transport(format!(
                    "Unexpected action when sending ForwardPacket: {:?}",
                    other
                )));
            }
        };

        todo!()
        // // 3. Send the packet with timeout
        // let timeout = self.outer_client.config.forward_timeout;
        // tokio::time::timeout(timeout, async {
        //     self.outer_client.try_send_packet(&forward_packet).await
        // })
        // .await
        // .map_err(|_| {
        //     LpClientError::Transport(format!("Forward packet timeout after {timeout:?}",))
        // })??;
        //
        // Ok(())
    }

    async fn receive_raw_packet(&mut self) -> Result<Vec<u8>, LpClientError>
    where
        S: LpTransport + Unpin,
    {
        // 1. Receive the packet with timeout
        let timeout = self.outer_client.config.forward_timeout;
        let response_packet = tokio::time::timeout(timeout, async {
            self.outer_client.try_receive_packet().await
        })
        .await
        .map_err(|_| {
            LpClientError::Transport(format!("Forward packet timeout after {timeout:?}",))
        })??;

        // 2. Decrypt via state machine (re-borrow)
        let state_machine = self.outer_client.state_machine_mut()?;
        let action = state_machine
            .process_input(LpInput::ReceivePacket(response_packet))
            .ok_or_else(|| LpClientError::transport("State machine returned no action"))?
            .map_err(|e| {
                LpClientError::Transport(format!("Failed to decrypt forward response: {e}"))
            })?;

        // 3. Extract decrypted response data
        let response_data = try_convert_forward_response(action)?;

        tracing::debug!(
            "Successfully received forward response from {} ({} bytes)",
            self.exit_address,
            response_data.len()
        );

        Ok(response_data)
    }
}

impl<'a, S> LpTransport for NestedConnection<'a, S>
where
    S: LpTransport + Unpin,
{
    #[allow(clippy::unimplemented)]
    async fn connect(_: SocketAddr) -> std::io::Result<Self> {
        // this really breaks the pattern and should be refactored
        // since this function should never be called
        unimplemented!("cannot establish nested connection without an outer client")
    }

    fn set_no_delay(&mut self, _: bool) -> std::io::Result<()> {
        Ok(())
    }

    async fn send_length_prefixed_packet(&mut self, packet_data: &[u8]) -> std::io::Result<()> {
        self.send_serialised_packet(packet_data)
            .await
            .map_err(io::Error::other)
    }

    async fn receive_length_prefixed_packet(&mut self) -> std::io::Result<Vec<u8>> {
        self.receive_raw_packet().await.map_err(io::Error::other)
    }
}
