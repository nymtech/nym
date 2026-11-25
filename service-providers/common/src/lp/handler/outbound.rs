// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Replies from the provider, frames to the node.
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
use nym_lp_data::{AddressedTimedData, PipelineInput};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_task::ShutdownToken;
use rand::{CryptoRng, Rng};
use tracing::{debug, warn};

use crate::embedded::ProviderOutputSender;
use crate::lp::handler::pipeline::SpOutboundPipeline;
use crate::lp::handler::PROVIDER_CHANNEL_BUFFER;

/// How often this direction is ticked.
///
/// The same millisecond the client's data handler uses. It bounds how late a scheduled packet can
/// be, so it is a privacy parameter once obfuscation is real, not merely a latency one.
pub(crate) const PIPELINE_TICKING_DURATION: Duration = Duration::from_millis(1);

/// Stands in for the destination the pipeline traits insist every payload carries.
///
/// A provider has exactly one place to send: the node it runs in, over a channel. The hop its packet
/// is *forwarded* to is named inside the frame, by node id, so no address is needed on the way down
/// and nothing ever reads this one.
const PLACEHOLDER_DESTINATION: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

/// What a provider hands over to be sent.
///
/// No transport flag: something that reaches here has already been routed to LP, by the path the
/// request it answers came in on.
pub struct ServiceProviderReply {
    pub data: Vec<u8>,
    pub to: ReplyTarget,
}

/// Who a reply goes to.
pub enum ReplyTarget {
    /// An address the sender named in its request, routed to like any other recipient.
    Recipient(Box<Recipient>),

    /// The sender's tag, to be answered through the reply SURBs it attached.
    SenderTag(AnonymousSenderTag),
}

pub(crate) struct SpOutbound<R> {
    /// Where a message is chunked, sphinx-wrapped and framed.
    pipeline: SpOutboundPipeline<R>,

    /// What the provider wants sent.
    from_provider: tokio::sync::mpsc::Receiver<ServiceProviderReply>,

    /// Where finished frames go, for the node to forward.
    to_node: ProviderOutputSender,

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
        to_node: ProviderOutputSender,
    ) -> Self {
        SpOutbound {
            pipeline,
            from_provider,
            to_node,
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
    /// Returns whether to keep going: not once the provider has gone, since nothing more will come
    /// to wrap.
    fn tick(&mut self, now: Instant) -> bool {
        let inputs = self.waiting_inputs();

        // ticked even with nothing to wrap: it is how the reliability and obfuscation stages get to
        // emit on their own schedule rather than only in response to something the provider said
        if inputs.is_empty() {
            self.wrap(None, now);
        }
        for input in inputs {
            self.wrap(Some(input), now);
        }

        if !self.release_due(now) {
            return false;
        }

        if self.provider_gone() {
            debug!("LP provider outbound: the provider has gone");
            return false;
        }

        true
    }

    /// What the provider has queued, ready for the pipeline.
    ///
    /// Everything waiting, not one reply per tick - which would cap this direction at a reply a
    /// millisecond. Capped at the channel's depth all the same, so a full channel empties in one
    /// tick but a provider that keeps refilling it cannot hold up the releases after it.
    fn waiting_inputs(&mut self) -> Vec<PipelineInput<LpOutboundOptions>> {
        std::iter::from_fn(|| self.from_provider.try_recv().ok())
            .take(PROVIDER_CHANNEL_BUFFER)
            // Temporary filter, while SURBs do not exist, we allow direct reply for testing's sake
            .filter_map(|reply| match reply.to {
                ReplyTarget::Recipient(recipient) => Some(PipelineInput::new(
                    reply.data,
                    LpOutboundOptions {
                        recipient: *recipient,
                    },
                    PLACEHOLDER_DESTINATION,
                )),
                // TODO: answer through the sender's reply SURBs, which this pipeline does not keep
                // yet - so every reply by tag is dropped until it does
                ReplyTarget::SenderTag(tag) => {
                    warn!(
                        "LP provider outbound: dropping a {} byte reply to {tag}: no reply SURBs on the LP path yet",
                        reply.data.len()
                    );
                    None
                }
            })
            .collect()
    }

    /// Whether the provider has dropped its side and everything it sent has been taken.
    fn provider_gone(&self) -> bool {
        self.from_provider.is_closed() && self.from_provider.is_empty()
    }

    /// Run one input, or none, through the pipeline, keeping what comes out for release.
    fn wrap(&mut self, input: Option<PipelineInput<LpOutboundOptions>>, now: Instant) {
        match self.pipeline.process(input, now) {
            Ok(frames) => self.release_buffer.extend(frames),
            Err(err) => warn!("LP provider outbound: could not wrap a message: {err}"),
        }
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
            match self.to_node.try_send(frame.data.data) {
                Ok(()) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    warn!("LP provider outbound: the node is not keeping up, dropping a frame")
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    debug!("LP provider outbound: the node closed the channel");
                    return false;
                }
            }
        }

        true
    }
}
