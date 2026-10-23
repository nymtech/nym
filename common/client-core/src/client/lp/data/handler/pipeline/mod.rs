// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The two ends of the LP data plane: bytes in, bytes out.
//!
//! Both are built from the trait stacks in [`nym_lp_data`], so both are plain synchronous state
//! machines that run on a blocking worker. Neither knows about channels, ticks or scheduling - that
//! is the handler's, and it is what keeps these testable on their own.
//!
//! ```text
//! outbound   bytes -> chunk -> reliability -> obfuscation -> routing security -> frame -> transport
//! inbound            transport -> unframe  -> routing security -> reassembly -> bytes
//! ```
//!

pub mod inbound;
pub mod outbound;

#[cfg(test)]
mod tests;

pub(crate) use inbound::LpInboundPipeline;
pub(crate) use outbound::{LpOutboundOptions, LpOutboundPipeline};
