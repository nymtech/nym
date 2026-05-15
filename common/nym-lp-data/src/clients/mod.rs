// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod driver;
pub mod helpers;
pub mod traits;
pub mod types;

/// Per-message pipeline configuration carried alongside every payload.
///
/// Each pipeline stage (reliability, routing security, obfuscation) is optional
/// and toggled per-message by the corresponding accessor.  The next-hop
/// destination is also resolved from the options so that addressing is decided
/// before the payload reaches [`Framing`].
///
/// # Type Parameters
/// - `NdId`: addressing type used to identify the next-hop destination.
///
/// [`Framing`]: crate::common::traits::Framing
pub trait InputOptions<NdId>: Clone {
    /// Whether reliability encoding (e.g. SURB ACKs) should be applied.
    fn reliability(&self) -> bool;
    /// Whether routing-security encryption (e.g. Sphinx) should be applied.
    fn routing_security(&self) -> bool;
    /// Whether obfuscation (e.g. cover traffic) should be applied.
    fn obfuscation(&self) -> bool;

    /// Identifier of the next-hop node this message should be sent to.
    fn next_hop(&self) -> NdId;
}
