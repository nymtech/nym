// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP Data Handler - UDP listener for LP data plane (port 51264)
//!
//! This module handles the data plane for LP clients that have completed registration
//! via the control plane (TCP:41264). LP-wrapped Sphinx packets arrive here, get
//! decrypted, and are forwarded into the mixnet.
//!
//! # Packet Flow
//!
//! ```text
//! LP Client → UDP:51264 → LP Data Handler → Mixnet Entry
//!           LP(Sphinx)      decrypt LP      forward Sphinx
//! ```
//!

use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::SharedLpDataState;
use nym_lp::packet::OuterHeader;
use nym_metrics::inc;
use std::net::SocketAddr;
use tracing::*;

/// LP Data Handler for UDP data plane
pub struct LpDataHandler {
    /// State used for handling received requests
    #[allow(dead_code)]
    state: SharedLpDataState,
}

impl LpDataHandler {
    /// Create a new LP data handler
    pub fn new(state: SharedLpDataState) -> Self {
        Self { state }
    }

    /// Handle a single UDP packet
    ///
    /// # Packet Processing Steps
    /// 1. Parse LP header to get receiver_idx (for routing)
    /// 2. Look up session state machine by receiver_idx
    /// 3. Process packet through state machine (handles decryption + replay protection)
    /// 4. Forward decrypted Sphinx packet to mixnet
    ///
    /// # Security
    /// The state machine's `process_input()` method handles replay protection by:
    /// - Checking packet counter against receiving window
    /// - Marking counter as used after successful decryption
    ///
    /// This prevents replay attacks where captured packets are re-sent.
    pub(crate) async fn handle_packet(
        &self,
        packet: &[u8],
        src_addr: SocketAddr,
    ) -> Result<(), LpHandlerError> {
        inc!("lp_data_packets_received");

        let _ = OuterHeader::parse(packet)?;
        trace!(
            "received {} bytes from {src_addr} on the unimplemented LP Data endpoint",
            packet.len()
        );

        Err(LpHandlerError::UnimplementedDataChannel)
        // leave old code for future reference

        //
        // // Step 1: Parse LP header (always cleartext for routing)
        // let header = nym_lp::codec::parse_lp_header_only(packet).map_err(|e| {
        //     LpHandlerError::LpProtocolError(format!("Failed to parse LP header: {}", e))
        // })?;
        //
        // let receiver_idx = header.receiver_idx;
        // let counter = header.counter;
        // let len = packet.len();
        //
        // trace!("LP data packet from {src_addr} (receiver_idx={receiver_idx}, counter={counter}, len={len})");
        //
        // // Step 2: Look up session state machine by receiver_idx (mutable for state updates)
        // let mut state_entry = self
        //     .state
        //     .session_states
        //     .get_mut(&receiver_idx)
        //     .ok_or_else(|| {
        //         inc!("lp_data_unknown_session");
        //         LpHandlerError::LpProtocolError(format!(
        //             "Unknown session for receiver_idx {receiver_idx}"
        //         ))
        //     })?;
        //
        // // Update last activity timestamp
        // state_entry.value().touch();
        //
        // // Step 3: Get outer AEAD key for packet parsing
        // let outer_key = state_entry
        //     .value()
        //     .state
        //     .session()
        //     .map_err(|e| LpHandlerError::LpProtocolError(format!("Session error: {e}")))?
        //     .outer_aead_key();
        //
        // // Parse full packet with outer AEAD decryption
        // let lp_packet = nym_lp::codec::parse_lp_packet(packet, Some(outer_key)).map_err(|e| {
        //     inc!("lp_data_decrypt_errors");
        //     LpHandlerError::LpProtocolError(format!("Failed to decrypt LP packet: {}", e))
        // })?;
        //
        // // Step 4: Process packet through state machine
        // // This handles:
        // // - Replay protection (counter check + mark)
        // // - Inner Noise decryption
        // // - Subsession handling if applicable
        // let state_machine = &mut state_entry.value_mut().state;
        //
        // let action = state_machine
        //     .process_input(LpInput::ReceivePacket(lp_packet))
        //     .ok_or_else(|| {
        //         LpHandlerError::LpProtocolError("State machine returned no action".to_string())
        //     })?
        //     .map_err(|e| {
        //         inc!("lp_data_state_machine_errors");
        //         LpHandlerError::LpProtocolError(format!("State machine error: {}", e))
        //     })?;
        //
        // // Release session lock before forwarding
        // drop(state_entry);
        //
        // // Step 5: Handle the action from state machine
        // match action {
        //     LpAction::DeliverData(data) => {
        //         // Decrypted application data - forward as Sphinx packet
        //         self.forward_sphinx_packet(&data.content).await?;
        //         inc!("lp_data_packets_forwarded");
        //         Ok(())
        //     }
        //     LpAction::SendPacket(_response_packet) => {
        //         // UDP is connectionless - we can't send responses back easily
        //         // For subsession rekeying, the client should use TCP control plane
        //         debug!(
        //             "Ignoring SendPacket action on UDP (receiver_idx={receiver_idx}) - use TCP for rekeying",
        //         );
        //         inc!("lp_data_ignored_send_actions");
        //         Ok(())
        //     }
        //     other => {
        //         warn!(
        //             "Unexpected action on UDP data plane from {}: {:?}",
        //             src_addr, other
        //         );
        //         inc!("lp_data_unexpected_actions");
        //         Err(LpHandlerError::LpProtocolError(format!(
        //             "Unexpected state machine action on UDP: {:?}",
        //             other
        //         )))
        //     }
        // }
    }
}
