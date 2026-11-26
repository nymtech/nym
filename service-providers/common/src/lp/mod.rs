// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! How a service provider embedded in a nym-node talks over the Lewes Protocol.
//!
//! A provider inside its node is not a client, and this is what it uses instead of one: a pipeline
//! pair that routes what it sends and peels what arrives, exchanging sphinx with the node over
//! channels rather than a socket. What it replaces, eventually, is the whole mixnet
//! client a provider builds today - there is no session to establish, no gateway to register with,
//! no acknowledgements to track and no cover traffic to emit, because the peer is the same process.
//!
//! ```text
//! outbound   plaintext -> chunk -> sphinx route -> frame (whole)  -> channel -> node
//! inbound    node -> channel -> peel final hop -> reassemble      -> plaintext
//! ```
//!
//! One implementation, one instance per provider: the peel needs *that* provider's keys and the
//! reassembler holds *that* provider's half-finished messages, so there is nothing here that three
//! providers could usefully share an instance of.

pub mod error;
pub mod handler;

use std::sync::Arc;

use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_client_core::config::DebugConfig;
use nym_crypto::asymmetric::x25519;
use nym_crypto::rng::os_rng;
use nym_pemstore::KeyPairPath;
use nym_task::ShutdownTracker;

use crate::embedded::EmbeddedLp;
use crate::lp::error::LpProviderError;
use crate::lp::handler::pipeline::{SpInboundPipeline, SpOutboundPipeline};
use crate::lp::handler::{ProviderLink, SpLpDataSetup};

/// Build a hosted provider's LP pipelines, start them, and hand back the provider's ends.
///
/// The LP counterpart of building a mixnet client from the host's transceiver. The pipelines are
/// built by the provider rather than by its host because the key the inbound half peels with is the
/// provider's own: on the LP path the provider is the final sphinx hop, so nothing else in the
/// process holds what opens that layer.
pub fn start_lp_data_plane(
    lp: EmbeddedLp,
    paths: &CommonClientPaths,
    debug_config: DebugConfig,
    shutdown: &ShutdownTracker,
    provider: &'static str,
) -> Result<ProviderLink, LpProviderError> {
    let encryption_keys: x25519::KeyPair = nym_pemstore::load_keypair(&KeyPairPath::new(
        paths.keys.private_encryption_key().to_path_buf(),
        paths.keys.public_encryption_key().to_path_buf(),
    ))
    .map_err(|source| LpProviderError::UnreadableKeys { provider, source })?;

    let (plane, channels) = SpLpDataSetup::new(
        SpInboundPipeline::new(Arc::new(encryption_keys)),
        SpOutboundPipeline::new(os_rng(), debug_config, lp.topology),
        lp.from_node,
        lp.to_node,
        lp.inbound_workers,
    );
    plane.start_tasks(shutdown, provider);

    Ok(channels)
}
