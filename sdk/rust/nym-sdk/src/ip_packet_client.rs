// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! IPR helpers for [`IpMixStream`](crate::ipr_wrapper::IpMixStream).
//!
//! - **Discovery** (`discovery`): queries the Nym API for IPR-enabled exit gateways.
//! - **Response handling** (`listener`): thin wrappers around
//!   [`nym_ip_packet_requests::response_helpers`] that add version checking
//!   and error mapping for SDK use.

pub mod discovery;
pub mod listener;

pub use listener::{handle_ipr_response, MixnetMessageOutcome};

// Re-export the currently used version
pub use nym_ip_packet_requests::v9 as current;
