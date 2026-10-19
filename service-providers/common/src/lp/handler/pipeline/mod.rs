// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The two ends of a provider's data plane: bytes in, bytes out.
//!
//! Both are built from the trait stacks in [`nym_lp_data`], so both are plain synchronous state
//! machines. Neither knows about channels or ticks - that is the handler's, and it is what keeps
//! these testable on their own, which [`tests`] does.
//!
//! ```text
//! outbound   bytes -> chunk -> reliability -> obfuscation -> routing security -> frame -> transport
//! inbound                                                    routing security -> reassembly -> bytes
//! ```
//!
//! The stages are the client's, and named for them, but the bottom two differ in each direction and
//! for the same reason: a provider's peer is in its own process, so there is no wire under either of
//! them. See each module for what that costs and what it does not.

pub mod inbound;
pub mod outbound;

#[cfg(test)]
mod tests;

pub use inbound::SpInboundPipeline;
pub use outbound::SpOutboundPipeline;
