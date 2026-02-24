// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use crate::replay::ReceivingKeyCounterValidator;
use nym_lp_packet::LpPacket;
#[allow(dead_code)]
pub(crate) const UDP_HEADER_LEN: usize = 8;
#[allow(dead_code)]
pub(crate) const IP_HEADER_LEN: usize = 40; // v4 - 20, v6 - 40
#[allow(dead_code)]
pub(crate) const MTU: usize = 1500;
#[allow(dead_code)]
pub(crate) const UDP_OVERHEAD: usize = UDP_HEADER_LEN + IP_HEADER_LEN;
#[allow(dead_code)]
pub(crate) const UDP_PAYLOAD_SIZE: usize = MTU - UDP_OVERHEAD;

pub use nym_lp_packet::version;

pub trait LpPacketReplayExt {
    /// Validate packet counter against a replay protection validator
    ///
    /// This performs a quick check to see if the packet counter is valid before
    /// any expensive processing is done.
    fn validate_counter(&self, validator: &ReceivingKeyCounterValidator) -> Result<(), LpError>;

    /// Mark packet as received in the replay protection validator
    ///
    /// This should be called after a packet has been successfully processed.
    fn mark_received(&self, validator: &mut ReceivingKeyCounterValidator) -> Result<(), LpError>;
}

impl LpPacketReplayExt for LpPacket {
    /// Validate packet counter against a replay protection validator
    ///
    /// This performs a quick check to see if the packet counter is valid before
    /// any expensive processing is done.
    fn validate_counter(&self, validator: &ReceivingKeyCounterValidator) -> Result<(), LpError> {
        validator.will_accept_branchless(self.header().outer.counter)?;
        Ok(())
    }

    /// Mark packet as received in the replay protection validator
    ///
    /// This should be called after a packet has been successfully processed.
    fn mark_received(&self, validator: &mut ReceivingKeyCounterValidator) -> Result<(), LpError> {
        validator.mark_did_receive_branchless(self.header().outer.counter)?;
        Ok(())
    }
}
