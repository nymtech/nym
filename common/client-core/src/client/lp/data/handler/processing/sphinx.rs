// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A sphinx packet addressed to this client.
//!
//! The client is the last sphinx hop on the LP path, so peeling the packet yields a bare
//! [`Fragment`] - no SURB-ack and no payload wrapper. See the [outbound pipeline's module
//! docs](crate::client::lp::data::handler::pipeline::outbound) for why.

use std::sync::Mutex;

use nym_lp_data::TimedPayload;
use nym_sphinx::chunking::fragment::Fragment;
use nym_sphinx::chunking::reconstruction::MessageReconstructor;
use nym_sphinx::message::PaddedMessage;
use nym_sphinx::{ProcessedPacketData, SphinxPacket};
use tracing::trace;

use crate::client::lp::data::handler::error::LpDataHandlerError;
use nym_crypto::asymmetric::x25519;

/// Take a sphinx packet down to the message bytes it was carrying, if it completes one.
///
/// `Ok(None)` is the ordinary outcome for a packet that legitimately yields nothing: cover traffic,
/// or a fragment that is not the last of its message.
pub(crate) fn process(
    encryption_keys: &x25519::KeyPair,
    reconstructor: &Mutex<MessageReconstructor>,
    payload: TimedPayload,
) -> Result<Option<Vec<u8>>, LpDataHandlerError> {
    let plaintext = recover_plaintext(encryption_keys, &payload.data)?;

    // nothing sends cover traffic on LP yet; this is the stage that would have to notice when
    // something does
    if nym_sphinx::cover::is_cover(&plaintext) {
        trace!("LP inbound: dropping a cover message");
        return Ok(None);
    }

    reassemble(reconstructor, &plaintext)
}

/// Feed one fragment to the reassembler, and unpad the message if that was the last of it.
///
/// `Ok(None)` means the message is still missing fragments. Whichever caller supplies the last one
/// is the one handed the result, which is what lets several workers share one reconstructor.
fn reassemble(
    reconstructor: &Mutex<MessageReconstructor>,
    plaintext: &[u8],
) -> Result<Option<Vec<u8>>, LpDataHandlerError> {
    let fragment = Fragment::try_from_bytes(plaintext)
        .map_err(|source| LpDataHandlerError::other(format!("malformed fragment: {source}")))?;

    // SAFETY: the only thing held across this lock is a map insert; nothing here can panic while
    // holding it, so it cannot be poisoned by our own code
    #[expect(clippy::unwrap_used)]
    let Some(reconstructed) = reconstructor.lock().unwrap().insert_new_fragment(fragment) else {
        return Ok(None);
    };

    let message = PaddedMessage::from(reconstructed.0)
        .remove_padding()
        .map_err(|source| {
            LpDataHandlerError::other(format!("could not remove padding: {source}"))
        })?;

    Ok(Some(message.into_inner_data()))
}

/// Peel the sphinx layer this client is the final hop of.
fn recover_plaintext(
    encryption_keys: &x25519::KeyPair,
    packet: &[u8],
) -> Result<Vec<u8>, LpDataHandlerError> {
    let sphinx_packet = SphinxPacket::from_bytes(packet)?;

    let processed = sphinx_packet.process(encryption_keys.private_key().as_ref())?;

    match processed.data {
        // the LP path ends here: a client is nobody's next hop
        ProcessedPacketData::ForwardHop { .. } => Err(LpDataHandlerError::ForwardHop),
        ProcessedPacketData::FinalHop { payload, .. } => Ok(payload.recover_plaintext()?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::{ed25519, x25519};
    use nym_sphinx::addressing::clients::Recipient;
    use nym_sphinx::message::NymMessage;
    use nym_sphinx::params::PacketSize;
    use nym_sphinx::routing::generate_hop_delays;
    use nym_sphinx::{Node as SphinxNode, NymPacket};
    use rand::rngs::OsRng;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// A client, and the keys its pipeline peels packets with.
    struct TestClient {
        recipient: Recipient,
        encryption_keys: Arc<x25519::KeyPair>,
    }

    impl TestClient {
        fn new() -> Self {
            let encryption_keys = Arc::new(x25519::KeyPair::new(&mut OsRng));
            let identity = ed25519::KeyPair::new(&mut OsRng);
            let gateway = ed25519::KeyPair::new(&mut OsRng);

            let recipient = Recipient::new(
                *identity.public_key(),
                *encryption_keys.public_key(),
                *gateway.public_key(),
            );

            TestClient {
                recipient,
                encryption_keys,
            }
        }

        /// What arrives on the wire for this client: a sphinx packet it is the last hop of, built
        /// the way [`prepare_chunk_for_lp`] builds one.
        ///
        /// The route is the recipient alone. Mix hops in front change nothing about the final-hop
        /// processing under test, and each would need a node to route through.
        ///
        /// [`prepare_chunk_for_lp`]: nym_sphinx::preparer::MessagePreparer::prepare_chunk_for_lp
        fn packet_carrying(&self, fragment: Vec<u8>) -> TimedPayload {
            let route = vec![SphinxNode::new(
                self.recipient.as_sphinx_hop().try_into().unwrap(),
                (*self.recipient.encryption_key()).into(),
            )];
            let delays = generate_hop_delays(Duration::from_millis(1), route.len());

            let packet = NymPacket::sphinx_build(
                false,
                PacketSize::RegularPacket.payload_size(),
                fragment,
                &route,
                &self.recipient.as_sphinx_destination(),
                &delays,
            )
            .unwrap();

            TimedPayload::new(Instant::now(), packet.to_bytes().unwrap())
        }
    }

    /// The bytes an LP payload actually carries: a bare `Fragment`, with no SURB-ack and no
    /// NymPayload wrapper, exactly as `prepare_chunk_for_lp` produces them.
    fn lp_payloads(data: Vec<u8>) -> Vec<Vec<u8>> {
        let message = NymMessage::new_plain(data);
        let per_packet = message.available_sphinx_plaintext_per_packet(PacketSize::RegularPacket);

        message
            .pad_to_full_packet_lengths(per_packet)
            .split_into_fragments(&mut OsRng, per_packet)
            .into_iter()
            .map(|fragment| fragment.into_bytes())
            .collect()
    }

    #[test]
    fn a_multi_packet_message_comes_back_whole() {
        let client = TestClient::new();
        let original = vec![42u8; 10 * 1024];
        let payloads = lp_payloads(original.clone());
        assert!(
            payloads.len() > 1,
            "the point of this test is a message that had to be split"
        );

        let reconstructor = Mutex::new(MessageReconstructor::new());

        // arrival order is whatever the network felt like
        let mut recovered = None;
        for payload in payloads.into_iter().rev() {
            assert!(
                recovered.is_none(),
                "a message completed before its last fragment"
            );
            recovered = process(
                &client.encryption_keys,
                &reconstructor,
                client.packet_carrying(payload),
            )
            .unwrap();
        }

        assert_eq!(Some(original), recovered);
    }

    #[test]
    fn fragments_scattered_across_workers_still_reassemble() {
        let client = TestClient::new();
        let original = vec![7u8; 10 * 1024];
        let payloads = lp_payloads(original.clone());

        // what round-robin dispatch does: consecutive fragments land on different workers, each
        // holding its own pipeline. Only the shared reconstructor lets them find each other.
        let shared = Arc::new(Mutex::new(MessageReconstructor::new()));
        let workers = [
            shared.clone(),
            shared.clone(),
            shared.clone(),
            shared.clone(),
        ];

        let mut recovered = None;
        for (i, payload) in payloads.into_iter().enumerate() {
            let out = process(
                &client.encryption_keys,
                &workers[i % workers.len()],
                client.packet_carrying(payload),
            )
            .unwrap();

            if out.is_some() {
                assert!(
                    recovered.is_none(),
                    "more than one worker completed the message"
                );
                recovered = out;
            }
        }

        assert_eq!(Some(original), recovered);
    }

    /// The property that makes the LP payload format its own thing: what comes out of the sphinx
    /// layer is the fragment itself, with nothing prepended.
    #[test]
    fn the_sphinx_payload_is_a_bare_fragment() {
        let client = TestClient::new();
        let payload = lp_payloads(vec![3u8; 128]).pop().unwrap();

        let recovered = recover_plaintext(
            &client.encryption_keys,
            &client.packet_carrying(payload.clone()).data,
        )
        .unwrap();

        assert_eq!(payload, recovered);
    }

    #[test]
    fn a_packet_carrying_something_that_is_not_a_fragment_is_an_error() {
        let client = TestClient::new();
        let reconstructor = Mutex::new(MessageReconstructor::new());

        let packet = client.packet_carrying(vec![0u8; 4]);

        assert!(process(&client.encryption_keys, &reconstructor, packet).is_err());
    }

    #[test]
    fn a_packet_someone_else_is_the_last_hop_of_cannot_be_peeled() {
        let client = TestClient::new();
        let someone_else = TestClient::new();
        let reconstructor = Mutex::new(MessageReconstructor::new());

        let packet = someone_else.packet_carrying(lp_payloads(vec![1u8; 128]).pop().unwrap());

        assert!(process(&client.encryption_keys, &reconstructor, packet).is_err());
    }
}
