// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Taking a packet off the wire and handing back the bytes it carried.
//!
//! The mirror of [`outbound`](super::outbound), stage for stage:
//! transport unwrap, unframing, sphinx, reassembly.

use crate::client::lp::data::handler::error::LpDataHandlerError;
use crate::client::lp::data::handler::messages::ClientMessage;
use crate::client::lp::data::handler::processing;
use crate::client::lp::data::shared::SharedLpDataState;
use nym_crypto::asymmetric::x25519;

use nym_lp_data::clients::traits::ClientUnwrappingPipeline;
use nym_lp_data::common::traits::{FramingUnwrap, TransportUnwrap, WireUnwrappingPipeline};
use nym_lp_data::fragmentation::fragment::Fragment as LpFragment;
use nym_lp_data::fragmentation::reconstruction::MessageReconstructor as LpFrameReconstructor;
use nym_lp_data::packet::frame::LpFrameKind;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_lp_data::{TimedData, TimedPayload};
use nym_sphinx::chunking::reconstruction::MessageReconstructor;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::warn;

/// Takes a packet off the wire and hands back the message bytes it was carrying.
///
/// One of these per worker, so several run at once, each a clone of the one the client was built
/// with. Every field is a handle, so cloning is what makes the sharing below happen: the sessions a
/// packet is decrypted on, and the reconstructors a fragment is inserted into, are the same ones for
/// every worker.
#[derive(Clone)]
pub(crate) struct LpInboundPipeline {
    /// The sessions a packet is decrypted on, shared with the outbound direction that encrypts on
    /// them.
    shared_state: Arc<SharedLpDataState>,

    /// This client's own keys, which the sphinx layer is peeled with.
    ///
    /// Only this direction needs them: outbound routes to recipients with their public keys, and
    /// never with ours.
    encryption_keys: Arc<x25519::KeyPair>,

    /// Where the frames carrying one sphinx packet accumulate.
    ///
    /// Shares its own state internally, so every worker holds the same one.
    frame_reconstructor: LpFrameReconstructor,

    /// Where the fragments of one message accumulate.
    ///
    /// Shared across the pool because round-robin dispatch scatters them across every worker;
    /// whichever inserts the last one is handed the result.
    reconstructor: Arc<Mutex<MessageReconstructor>>,
}

impl LpInboundPipeline {
    pub(crate) fn new(
        shared_state: Arc<SharedLpDataState>,
        encryption_keys: Arc<x25519::KeyPair>,
    ) -> Self {
        LpInboundPipeline {
            shared_state,
            encryption_keys,
            frame_reconstructor: LpFrameReconstructor::default(),
            reconstructor: Arc::new(Mutex::new(MessageReconstructor::new())),
        }
    }
}

impl TransportUnwrap<EncryptedLpPacket> for LpInboundPipeline {
    type Frame = LpFrame;
    type Error = LpDataHandlerError;

    fn packet_to_frame(
        &mut self,
        packet: EncryptedLpPacket,
        timestamp: Instant,
    ) -> Result<TimedData<Self::Frame>, Self::Error> {
        Ok(TimedData::new(
            timestamp,
            self.shared_state.sessions.receive(packet)?,
        ))
    }
}

impl FramingUnwrap<ClientMessage> for LpInboundPipeline {
    type Frame = LpFrame;

    fn frame_to_message(
        &mut self,
        frame: TimedData<Self::Frame>,
    ) -> Option<(TimedPayload, ClientMessage)> {
        let timestamp = frame.timestamp;

        // a sphinx packet is larger than one frame, so it arrives as several - the frame we want is
        // the one they reassemble into, and it only exists once the last has landed
        let frame = match frame.data.kind() {
            LpFrameKind::FragmentedData => {
                let fragment = LpFragment::try_from(frame.data)
                    .inspect_err(|err| warn!("LP inbound: malformed frame fragment: {err}"))
                    .ok()?;

                self.frame_reconstructor
                    .insert_new_fragment(fragment, timestamp)?
                    .inspect_err(|err| warn!("LP inbound: could not rebuild a frame: {err}"))
                    .ok()?
            }
            _ => frame.data,
        };

        let kind = ClientMessage::from_frame_header(frame.header.clone())
            .inspect_err(|err| warn!("LP inbound: unusable frame: {err}"))
            .ok()?;

        Some((TimedPayload::new(timestamp, frame.content.into()), kind))
    }
}

impl WireUnwrappingPipeline<EncryptedLpPacket, ClientMessage> for LpInboundPipeline {}

impl ClientUnwrappingPipeline<EncryptedLpPacket, ClientMessage> for LpInboundPipeline {
    /// Hand the frame to whatever handles its kind.
    ///
    /// Every arm reports failure the same way, so a new [`ClientMessage`] variant is a new arm and a
    /// new [`processing`] module - see that module's docs.
    fn process_unwrapped(&mut self, payload: TimedPayload, kind: ClientMessage) -> Option<Vec<u8>> {
        let processed = match kind {
            ClientMessage::Sphinx(_metadata) => {
                // metadata for now only have key rotation info, which is irrelevant to clients
                processing::sphinx::process(&self.encryption_keys, &self.reconstructor, payload)
            }
        };

        processed
            .inspect_err(|err| warn!("LP inbound: {err}"))
            .ok()
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_lp_data::fragmentation::fragment::fragment_lp_message;
    use rand::rngs::OsRng;

    /// A sphinx packet does not fit in one LP frame, so the outbound side splits it and this side
    /// has to put it back - the seam `to_frame` and `frame_to_message` sit on either side of.
    #[test]
    fn a_frame_too_big_for_one_survives_being_split() {
        let original = LpFrame::new(LpFrameKind::SphinxPacket, vec![9u8; 3 * 1024]);

        let fragments = fragment_lp_message(&mut OsRng, original.clone(), 1024);
        assert!(
            fragments.len() > 1,
            "the point of this test is a frame that had to be split"
        );

        let reconstructor = LpFrameReconstructor::default();

        // arrival order is whatever the network felt like
        let mut rebuilt = None;
        for fragment in fragments.into_iter().rev() {
            assert!(rebuilt.is_none(), "a frame completed before its last piece");
            rebuilt = reconstructor
                .insert_new_fragment(fragment, Instant::now())
                .map(|res| res.unwrap());
        }

        assert_eq!(Some(original), rebuilt);
    }
}
