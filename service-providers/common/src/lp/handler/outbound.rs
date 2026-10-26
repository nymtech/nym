// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Replies from the provider, frames to the gateway.
//!
//! Everything the provider's outbound direction owns. Its counterpart is [`inbound`](super::inbound).
//!
//! # Why this one ticks
//!
//! Its pipeline has a reliability stage and an obfuscation stage, and both - once they are more
//! than the no-ops they are today - produce packets on a *schedule* rather than in response to
//! input: a retransmission is due when its timer says so, and cover traffic is due whether or not
//! anything was sent. So the pipeline is ticked even when nothing is waiting, which is what gives
//! those stages the chance to emit, and what comes out waits in a release buffer until its time
//! rather than going at once.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use nym_client_core::client::lp::data::handler::pipeline::outbound::LpOutboundOptions;
use nym_lp_data::clients::traits::ClientWrappingPipeline;
use nym_lp_data::packet::LpFrame;
use nym_lp_data::AddressedTimedData;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::ShutdownToken;
use rand::{CryptoRng, Rng};
use tracing::{debug, warn};

use crate::lp::handler::pipeline::SpOutboundPipeline;
use crate::lp::ServiceProviderOutputSender;

/// How often this direction is ticked.
///
/// The same millisecond the client's data handler uses. It bounds how late a scheduled packet can
/// be, so it is a privacy parameter once obfuscation is real, not merely a latency one.
pub(crate) const PIPELINE_TICKING_DURATION: Duration = Duration::from_millis(1);

/// Stands in for the destination the pipeline traits insist every payload carries.
///
/// A provider has exactly one place to send: the gateway in its own process, over a channel. The
/// hop its packet is *forwarded* to is named inside the frame, by node id, so no address is needed
/// on the way down and nothing ever reads this one.
const PLACEHOLDER_DESTINATION: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

/// What a provider hands over to be sent.
///
/// No transport flag: something that reaches here has already been routed to LP. What decides that
/// is the `legacy` flag the provider's loop set on the request this answers.
pub struct ServiceProviderReply {
    pub data: Vec<u8>,
    pub recipient: Recipient,
}

pub(crate) struct SpOutbound<R> {
    /// Where a message is chunked, sphinx-wrapped and framed.
    pipeline: SpOutboundPipeline<R>,

    /// What the provider wants sent.
    from_provider: tokio::sync::mpsc::Receiver<ServiceProviderReply>,

    /// Where finished frames go, for the gateway to forward.
    to_gateway: ServiceProviderOutputSender,

    /// Frames waiting for their release time.
    ///
    /// Empty as long as reliability and obfuscation are no-ops, since nothing else stamps a frame
    /// for later. It is here because they will not always be, and a direction whose output could
    /// only be sent immediately would have nowhere to put a retransmission or a cover packet.
    release_buffer: Vec<AddressedTimedData<LpFrame>>,
}

impl<R> SpOutbound<R>
where
    R: CryptoRng + Rng,
{
    pub(crate) fn new(
        pipeline: SpOutboundPipeline<R>,
        from_provider: tokio::sync::mpsc::Receiver<ServiceProviderReply>,
        to_gateway: ServiceProviderOutputSender,
    ) -> Self {
        SpOutbound {
            pipeline,
            from_provider,
            to_gateway,
            release_buffer: Vec::new(),
        }
    }

    pub(crate) fn run(mut self, shutdown: ShutdownToken) {
        while !shutdown.is_cancelled() {
            std::thread::sleep(PIPELINE_TICKING_DURATION);

            if !self.tick(Instant::now()) {
                break;
            }
        }

        debug!("LP provider outbound: stopping");
    }

    /// One turn: take what is waiting, wrap it, release what is due.
    ///
    /// Returns whether to keep going.
    fn tick(&mut self, now: Instant) -> bool {
        // `None` when nothing is waiting, which is the whole point of ticking: it is how the
        // reliability and obfuscation stages get to emit on their own schedule rather than only in
        // response to something the provider said.
        let waiting = self.from_provider.try_recv().ok().map(|reply| {
            (
                reply.data,
                LpOutboundOptions {
                    recipient: reply.recipient,
                },
                PLACEHOLDER_DESTINATION,
            )
        });

        match self.pipeline.process(waiting, now) {
            Ok(frames) => self.release_buffer.extend(frames),
            Err(err) => warn!("LP provider outbound: could not wrap a message: {err}"),
        }

        self.release_due(now)
    }

    /// Hand over the frames whose time has come, keeping the rest.
    fn release_due(&mut self, now: Instant) -> bool {
        if self.release_buffer.is_empty() {
            return true;
        }

        let (due, waiting) = self
            .release_buffer
            .drain(..)
            .partition(|frame| frame.data.timestamp <= now);
        self.release_buffer = waiting;

        for frame in due {
            match self.to_gateway.try_send(frame.data.data) {
                Ok(()) => {}
                Err(std::sync::mpsc::TrySendError::Full(_)) => {
                    warn!("LP provider outbound: the gateway is not keeping up, dropping a frame")
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                    debug!("LP provider outbound: the gateway closed the channel");
                    return false;
                }
            }
        }

        true
    }
}
