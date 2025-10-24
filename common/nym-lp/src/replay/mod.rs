// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Replay protection module for the Lewes Protocol.
//!
//! This module implements BoringTun-style replay protection to prevent
//! replay attacks and ensure packet ordering. It uses a bitmap-based
//! approach to track received packets and validate their sequence.

pub mod error;
pub mod simd;
pub mod validator;

pub use error::ReplayError;
pub use validator::ReceivingKeyCounterValidator;
