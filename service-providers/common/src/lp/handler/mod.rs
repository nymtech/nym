// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What drives a provider's pipeline pair: the two directions between it and its gateway.
//!
//! The counterpart of the client's `LpDataHandler`, and built the same way - a setup that is
//! constructed and then consumed by [`start_tasks`](SpLpDataSetup::start_tasks), owning one
//! direction per file. The difference is that the two directions here run independently rather than
//! off a shared tick: only [`outbound`] has anything to schedule. See each for why.

use nym_task::ShutdownTracker;
use rand::{CryptoRng, Rng};
use tracing::info;

use crate::lp::handler::inbound::SpInbound;
use crate::lp::handler::outbound::{ServiceProviderReply, SpOutbound};
use crate::lp::handler::pipeline::{SpInboundPipeline, SpOutboundPipeline};
use crate::lp::PipelineLink;

pub mod inbound;
pub mod outbound;
pub mod pipeline;

/// How many messages either direction may have outstanding towards the provider.
const PROVIDER_CHANNEL_BUFFER: usize = 100;

/// The provider's side of the link between it and its pipelines: plaintext in, replies out.
///
/// The seam the whole pair exists to serve, and the level above
/// [`GatewayLink`](crate::lp::GatewayLink) - what crosses here is what the pipelines have already
/// peeled. What a provider does with plaintext is its own, and is the same code whichever transport
/// delivered it: which one did is something its loop knows from the arm it read and tags the
/// request with, rather than anything carried across here.
///
/// The pipelines' side of this link has no name because nothing hands it out - it stays inside
/// [`SpLpDataSetup`].
pub struct ProviderLink {
    pub inbound: tokio::sync::mpsc::Receiver<Vec<u8>>,
    pub outbound: tokio::sync::mpsc::Sender<ServiceProviderReply>,
}

/// Everything a hosted provider needs to speak LP, built and not yet running.
pub struct SpLpDataSetup<R> {
    inbound: SpInbound,
    outbound: SpOutbound<R>,
}

impl<R> SpLpDataSetup<R>
where
    R: CryptoRng + Rng + Send + 'static,
{
    /// Wire a provider's pipelines to the ends of the link its gateway gave it.
    ///
    /// Hands back the seam the provider itself talks to, since nothing else can hold it - and
    /// something must, or the channels close and both directions stop.
    pub fn new(
        inbound: SpInboundPipeline,
        outbound: SpOutboundPipeline<R>,
        gateway: PipelineLink,
        inbound_workers: usize,
    ) -> (Self, ProviderLink) {
        let (to_provider, provider_inbound) = tokio::sync::mpsc::channel(PROVIDER_CHANNEL_BUFFER);
        let (provider_outbound, from_provider) =
            tokio::sync::mpsc::channel(PROVIDER_CHANNEL_BUFFER);

        let setup = SpLpDataSetup {
            inbound: SpInbound::new(inbound, gateway.from_gateway, to_provider, inbound_workers),
            outbound: SpOutbound::new(outbound, from_provider, gateway.to_gateway),
        };

        let provider = ProviderLink {
            inbound: provider_inbound,
            outbound: provider_outbound,
        };

        (setup, provider)
    }

    /// Run both directions until the channels to the gateway close.
    ///
    /// Blocking threads rather than async tasks: every stage in both pipelines is synchronous and
    /// CPU-bound - sphinx wrapping on the way out, peeling on the way in - so neither belongs on
    /// the runtime's cooperative threads. Inbound spawns several, one per worker plus its
    /// dispatcher; outbound is a single scheduler and stays one.
    pub fn start_tasks(self, shutdown_tracker: &ShutdownTracker, provider: &str) {
        self.inbound.start(shutdown_tracker);

        let outbound = self.outbound;
        let shutdown = shutdown_tracker.clone_shutdown_token();
        shutdown_tracker.spawn_blocking(move || outbound.run(shutdown));

        info!("started the LP data plane for the {provider}");
    }
}
