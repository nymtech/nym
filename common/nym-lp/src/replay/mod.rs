// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Replay protection module for the Lewes Protocol.
//!
//! This module implements BoringTun-style replay protection to prevent
//! replay attacks and ensure packet ordering. It uses a bitmap-based
//! approach to track received packets and validate their sequence.

use crate::LpError;
use nym_lp_data::packet::LpPacket;

pub mod error;
pub mod simd;
pub mod validator;

pub use error::ReplayError;
pub use validator::ReceivingKeyCounterValidator;

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
