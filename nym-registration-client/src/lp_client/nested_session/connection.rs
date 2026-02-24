// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::lp_client::helpers::{convert_forward_data, try_convert_forward_response};
use crate::{LpClientError, LpRegistrationClient};
use bytes::{BufMut, BytesMut};
use nym_lp::state_machine::{LpAction, LpInput};
use nym_lp::{EncryptedLpPacket, ExpectedResponseSize, ForwardPacketData, KEM};
use nym_lp_transport::traits::{HandshakeMessage, LpTransportChannel};
use nym_lp_transport::{LpHandshakeChannel, LpTransportError};
use std::io;
use std::net::SocketAddr;

/// Attempt to treat the inner client as a LP connection
pub struct NestedConnection<'a, S> {
    /// Exit gateway's LP address (e.g., "2.2.2.2:41264")
    pub(crate) exit_address: SocketAddr,

    // exact mechanisms of determining this value are TBD
    pub(crate) outer_client: &'a mut LpRegistrationClient<S>,
}

impl<'a, S> NestedConnection<'a, S> {
    fn prepare_handshake_message<M: HandshakeMessage>(
        &self,
        message: M,
        handshake_kem: KEM,
    ) -> Result<ForwardPacketData, LpClientError> {
        let Some(response_size) = message.response_size(handshake_kem) else {
            // this should NEVER happen for an initiator
            return Err(LpClientError::Other("unexpected empty response".into()));
        };

        let expected_size = ExpectedResponseSize::Handshake(response_size as u32);

        Ok(ForwardPacketData::new(
            self.exit_address,
            expected_size,
            message.into_bytes(),
        ))
    }

    fn prepare_transport_message(&self, packet: &EncryptedLpPacket) -> ForwardPacketData {
        let mut buf = BytesMut::new();
        let len = packet.encoded_length() as u32;
        buf.put_u32_le(len);
        packet.encode(&mut buf);
        ForwardPacketData::new(
            self.exit_address,
            ExpectedResponseSize::Transport,
            buf.freeze().into(),
        )
    }

    async fn send_forward_packet(&mut self, data: ForwardPacketData) -> Result<(), LpClientError>
    where
        S: LpTransportChannel + LpHandshakeChannel + Unpin,
    {
        tracing::debug!(
            "Sending ForwardPacket to {} ({} inner bytes, persistent connection)",
            data.target_lp_address,
            data.inner_packet_bytes.len()
        );

        // 1. Serialize the ForwardPacketData
        let input = convert_forward_data(data)?;

        // 2. Encrypt and prepare packet via state machine
        let state_machine = self.outer_client.state_machine_mut()?;

        let action = state_machine
            .process_input(input)
            .ok_or(LpClientError::UnexpectedStateMachineHalt)??;

        let forward_packet = match action {
            LpAction::SendPacket(packet) => packet,
            action => return Err(LpClientError::UnexpectedStateMachineAction { action }),
        };

        // 3. Send the packet with timeout
        let timeout = self.outer_client.config.forward_timeout;
        tokio::time::timeout(timeout, async {
            self.outer_client.try_send_packet(&forward_packet).await
        })
        .await
        .map_err(|_| LpClientError::ConnectionTimeout)??;

        Ok(())
    }

    async fn receive_forward_packet_data(&mut self) -> Result<Vec<u8>, LpClientError>
    where
        S: LpTransportChannel + LpHandshakeChannel + Unpin,
    {
        // 1. Receive the packet with timeout
        let timeout = self.outer_client.config.forward_timeout;
        let response_packet = tokio::time::timeout(timeout, async {
            self.outer_client.try_receive_packet().await
        })
        .await
        .map_err(|_| LpClientError::ConnectionTimeout)??;

        // 2. Decrypt via state machine (re-borrow)
        let state_machine = self.outer_client.state_machine_mut()?;
        let action = state_machine
            .process_input(LpInput::ReceivePacket(response_packet))
            .ok_or(LpClientError::UnexpectedStateMachineHalt)??;

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

impl<'a, S> LpHandshakeChannel for NestedConnection<'a, S>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
{
    #[allow(clippy::unimplemented)]
    async fn write_all_and_flush(&mut self, _: &[u8]) -> Result<(), LpTransportError> {
        // this is not being called instead we implement `send_handshake_message` directly
        unimplemented!()
    }

    #[allow(clippy::unimplemented)]
    async fn read_n_bytes(&mut self, _: usize) -> Result<Vec<u8>, LpTransportError> {
        // this is not being called instead we implement `receive_handshake_message` directly
        unimplemented!()
    }

    async fn send_handshake_message<M: HandshakeMessage>(
        &mut self,
        message: M,
        handshake_kem: KEM,
    ) -> Result<(), LpTransportError> {
        let forward_data = self
            .prepare_handshake_message(message, handshake_kem)
            .map_err(|err| LpTransportError::TransportSendFailure(err.to_string()))?;
        self.send_forward_packet(forward_data)
            .await
            .map_err(|err| LpTransportError::TransportSendFailure(err.to_string()))
    }

    async fn receive_handshake_message<M: HandshakeMessage>(
        &mut self,
        _: usize,
    ) -> Result<M, LpTransportError> {
        let data = self
            .receive_forward_packet_data()
            .await
            .map_err(|err| LpTransportError::TransportReceiveFailure(err.to_string()))?;
        M::try_from_bytes(data)
    }
}

impl<'a, S> LpTransportChannel for NestedConnection<'a, S>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
{
    #[allow(clippy::unimplemented)]
    async fn connect(_: SocketAddr) -> Result<Self, LpTransportError> {
        // this really breaks the pattern and should be refactored
        // since this function should never be called
        unimplemented!("cannot establish nested connection without an outer client")
    }

    fn set_no_delay(&mut self, _: bool) -> Result<(), LpTransportError> {
        Ok(())
    }

    async fn send_length_prefixed_transport_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<(), LpTransportError> {
        let packet = self.prepare_transport_message(packet);
        self.send_forward_packet(packet)
            .await
            .map_err(io::Error::other)
            .map_err(LpTransportError::send_failure)
    }

    async fn receive_length_prefixed_transport_bytes(
        &mut self,
    ) -> Result<Vec<u8>, LpTransportError> {
        self.receive_forward_packet_data()
            .await
            .map_err(|err| LpTransportError::TransportReceiveFailure(err.to_string()))
    }
}
