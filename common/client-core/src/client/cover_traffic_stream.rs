// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::mix_traffic::BatchMixMessageSender;
use crate::client::topology_control::TopologyAccessor;
use crate::config;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use nym_crypto::rng::{OsRng, os_rng};
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::cover::generate_loop_cover_packet;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_sphinx::utils::sample_poisson_duration;
use nym_statistics_common::clients::{ClientStatsSender, packet_statistics::PacketStatisticsEvent};
use rand::{CryptoRng, Rng, RngExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tracing::*;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::{Sleep, sleep};

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::{Sleep, sleep};

pub struct LoopCoverTrafficStream<R>
where
    R: CryptoRng + Rng,
{
    /// Key used to encrypt and decrypt content of an ACK packet.
    ack_key: Arc<AckKey>,

    /// Average delay an acknowledgement packet is going to get delayed at a single mixnode.
    average_ack_delay: Duration,

    /// Average delay a forward packet is going to get delayed at a single mixnode.
    average_packet_delay: Duration,

    /// Defines configuration options related to cover traffic.
    cover_traffic: config::CoverTraffic,

    /// Internal state, determined by `average_message_sending_delay`,
    /// used to keep track of when a next packet should be sent out.
    next_delay: Pin<Box<Sleep>>,

    /// Channel used for sending prepared nym packets to `MixTrafficController` that sends them
    /// out to the network without any further delays.
    mix_tx: BatchMixMessageSender,

    /// Represents full address of this client.
    our_full_destination: Recipient,

    /// Instance of a cryptographically secure random number generator.
    rng: R,

    /// Accessor to the common instance of network topology.
    topology_access: TopologyAccessor,

    /// Primary predefined packet size used for the loop cover messages.
    primary_packet_size: PacketSize,

    /// Optional secondary predefined packet size used for the loop cover messages.
    secondary_packet_size: Option<PacketSize>,

    packet_type: PacketType,

    stats_tx: ClientStatsSender,
}

impl<R> Stream for LoopCoverTrafficStream<R>
where
    R: CryptoRng + Rng + Unpin,
{
    // Item is only used to indicate we should create a new message rather than actual cover message
    // reason being to not introduce unnecessary complexity by having to keep state of topology
    // mutex when trying to acquire it. So right now the Stream trait serves as a glorified timer.
    // Perhaps this should be changed in the future.
    type Item = ();

    // r[impl cover.loop-exponential]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // it is not yet time to return a message
        if self.next_delay.as_mut().poll(cx).is_pending() {
            return Poll::Pending;
        };

        // we know it's time to send a message, so let's prepare delay for the next one
        // Get the `now` by looking at the current `delay` deadline
        let avg_delay = self.cover_traffic.loop_cover_traffic_average_delay;
        let next_poisson_delay = sample_poisson_duration(&mut self.rng, avg_delay);

        // The next interval value is `next_poisson_delay` after the one that just
        // yielded.
        let now = self.next_delay.deadline();
        let next = now + next_poisson_delay;
        self.next_delay.as_mut().reset(next);

        Poll::Ready(Some(()))
    }
}

// obviously when we finally make shared rng that is on 'higher' level, this should become
// generic `R`
impl LoopCoverTrafficStream<OsRng> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        ack_key: Arc<AckKey>,
        average_ack_delay: Duration,
        mix_tx: BatchMixMessageSender,
        our_full_destination: Recipient,
        topology_access: TopologyAccessor,
        traffic_config: config::Traffic,
        cover_config: config::CoverTraffic,
        stats_tx: ClientStatsSender,
    ) -> Self {
        let rng = os_rng();

        let next_delay = Box::pin(sleep(Default::default()));

        LoopCoverTrafficStream {
            ack_key,
            average_ack_delay,
            average_packet_delay: traffic_config.average_packet_delay,
            cover_traffic: cover_config,
            next_delay,
            mix_tx,
            our_full_destination,
            rng,
            topology_access,
            primary_packet_size: traffic_config.primary_packet_size,
            secondary_packet_size: traffic_config.secondary_packet_size,
            packet_type: traffic_config.packet_type,
            stats_tx,
        }
    }

    fn set_next_delay(&mut self, amount: Duration) {
        let next_delay = Box::pin(sleep(amount));
        self.next_delay = next_delay;
    }

    fn loop_cover_message_size(&mut self) -> PacketSize {
        let Some(secondary_packet_size) = self.secondary_packet_size else {
            return self.primary_packet_size;
        };

        let use_primary = self
            .rng
            .random_bool(self.cover_traffic.cover_traffic_primary_size_ratio);

        if use_primary {
            self.primary_packet_size
        } else {
            secondary_packet_size
        }
    }

    // r[impl cover.loop-own-address]
    // r[impl cover.rate-can-drop]
    async fn on_new_message(&mut self) {
        trace!("next cover message!");

        let cover_traffic_packet_size = self.loop_cover_message_size();
        trace!("the next loop cover message will be put in a {cover_traffic_packet_size} packet");

        // the ack is sent back to ourselves (and then ignored)
        let topology = match self
            .topology_access
            .try_get_valid_topology(&self.our_full_destination, Some(&self.our_full_destination))
        {
            Ok(topology) => topology,
            Err(err) => {
                warn!(
                    "We're not going to send any loop cover message this time, as the current topology seem to be invalid - {err}"
                );
                return;
            }
        };

        let cover_message = match generate_loop_cover_packet(
            &mut self.rng,
            &topology,
            &self.ack_key,
            &self.our_full_destination,
            self.average_ack_delay,
            self.average_packet_delay,
            cover_traffic_packet_size,
            self.packet_type,
        ) {
            Ok(cover_message) => cover_message,
            Err(err) => {
                warn!(
                    "Somehow failed to generate a loop cover message with a valid topology: {err}"
                );
                return;
            }
        };

        if let Err(err) = self.mix_tx.try_send(vec![cover_message]) {
            match err {
                TrySendError::Full(_) => {
                    // This isn't a problem, if the channel is full means we're already sending the
                    // max amount of messages downstream can handle.
                    tracing::trace!("Failed to send cover message - channel full");
                }
                TrySendError::Closed(_) => {
                    tracing::warn!("Failed to send cover message - channel closed");
                }
            }
        } else {
            self.stats_tx.report(
                PacketStatisticsEvent::CoverPacketSent(cover_traffic_packet_size.size()).into(),
            );
        }

        // JS: due to identical logical structure to OutQueueControl::on_message(), this is also
        // presumably required to prevent bugs in the future. Exact reason is still unknown to me.

        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::task::yield_now().await;
        }

        #[cfg(target_arch = "wasm32")]
        {
            tokio_with_wasm::task::yield_now().await;
        }
    }

    // it's fine if cover traffic stream task gets killed whilst processing next message
    #[allow(clippy::panic)]
    pub async fn run(&mut self) {
        if self.cover_traffic.disable_loop_cover_traffic_stream {
            // we should have never got here in the first place - the task should have never been created to begin with
            // so panic and review the code that lead to this branch
            panic!("attempted to run LoopCoverTrafficStream while config explicitly disabled it.")
        }

        // we should set initial delay only when we actually start the stream
        let sampled = sample_poisson_duration(
            &mut self.rng,
            self.cover_traffic.loop_cover_traffic_average_delay,
        );
        self.set_next_delay(sampled);

        while self.next().await.is_some() {
            self.on_new_message().await;
        }

        // this should never get triggered
        error!("cover traffic stream has been exhausted!")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use nym_crypto::asymmetric::{ed25519, x25519};
    use nym_sphinx::acknowledgements::identifier::recover_identifier;
    use nym_sphinx::acknowledgements::surb_ack::SurbAck;
    use nym_sphinx::chunking::fragment::COVER_FRAG_ID;
    use nym_sphinx::cover::LOOP_COVER_MESSAGE_PAYLOAD;
    use nym_sphinx::forwarding::packet::MixPacket;
    use nym_sphinx::receiver::{MessageReceiver, SphinxMessageReceiver};
    use nym_sphinx::{DestinationAddressBytes, NymPacket, ProcessedPacketData};
    use nym_task::ShutdownToken;
    use nym_topology::{
        CachedEpochRewardedSet, NymRouteProvider, NymTopology, NymTopologyMetadata, RoutingNode,
        SupportedRoles,
    };
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    /// One mix node on each layer and one gateway, with the private keys of every party.
    pub(crate) struct TestNetwork {
        pub(crate) topology: NymTopology,
        /// Sphinx keys of the layer 1, 2 and 3 mix nodes and of the gateway, in route order.
        pub(crate) node_keys: Vec<x25519::PrivateKey>,
        pub(crate) client_keys: Arc<x25519::KeyPair>,
        pub(crate) address: Recipient,
        pub(crate) ack_key: Arc<AckKey>,
    }

    impl TestNetwork {
        pub(crate) fn new() -> Self {
            let mut rng = ChaCha8Rng::seed_from_u64(42);
            let mut nodes = Vec::new();
            let mut node_keys = Vec::new();
            for node_id in 1..=4 {
                let sphinx_key = x25519::PrivateKey::new(&mut rng);
                nodes.push(RoutingNode {
                    node_id,
                    mix_host: format!("10.0.0.{node_id}:1789").parse().unwrap(),
                    entry: None,
                    identity_key: *ed25519::KeyPair::new(&mut rng).public_key(),
                    sphinx_key: sphinx_key.public_key(),
                    supported_roles: SupportedRoles {
                        mixnode: true,
                        mixnet_entry: true,
                        mixnet_exit: true,
                    },
                });
                node_keys.push(sphinx_key);
            }

            let mut rewarded_set = CachedEpochRewardedSet::default();
            rewarded_set.layer1.insert(1);
            rewarded_set.layer2.insert(2);
            rewarded_set.layer3.insert(3);
            rewarded_set.entry_gateways.insert(4);

            let client_keys = x25519::KeyPair::new(&mut rng);
            let address = Recipient::new(
                *ed25519::KeyPair::new(&mut rng).public_key(),
                *client_keys.public_key(),
                nodes[3].identity_key,
            );

            TestNetwork {
                topology: NymTopology::new(NymTopologyMetadata::default(), rewarded_set, nodes),
                node_keys,
                client_keys: Arc::new(client_keys),
                address,
                ack_key: Arc::new(AckKey::new(&mut rng)),
            }
        }

        pub(crate) fn topology_accessor(&self) -> TopologyAccessor {
            let accessor = TopologyAccessor::new(false);
            accessor.update_global_topology(Some(self.topology.clone()));
            accessor
        }

        pub(crate) fn loop_cover_packet(&self) -> MixPacket {
            generate_loop_cover_packet(
                &mut ChaCha8Rng::seed_from_u64(1),
                &NymRouteProvider::from(self.topology.clone()),
                &self.ack_key,
                &self.address,
                Duration::ZERO,
                Duration::ZERO,
                PacketSize::RegularPacket,
                PacketType::Mix,
            )
            .unwrap()
        }

        /// Removes every Sphinx layer with the node keys, as the mix nodes and the gateway do,
        /// and returns the destination and the plaintext of the final hop.
        pub(crate) fn unwrap_all_hops(
            &self,
            packet: NymPacket,
        ) -> (DestinationAddressBytes, Vec<u8>) {
            let mut packet = packet.to_sphinx_packet().unwrap();
            for key in &self.node_keys {
                let key = nym_sphinx::PrivateKey::from(key.to_bytes());
                match packet.process(&key).unwrap().data {
                    ProcessedPacketData::ForwardHop {
                        next_hop_packet, ..
                    } => packet = next_hop_packet,
                    ProcessedPacketData::FinalHop {
                        destination,
                        payload,
                        ..
                    } => return (destination, payload.recover_plaintext().unwrap()),
                }
            }
            panic!("the packet did not reach its final hop")
        }

        /// Splits the final hop plaintext of a regular packet into the SURB-ACK and the message,
        /// as the gateway does.
        pub(crate) fn split_surb_ack(mut hop_data: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
            let message = hop_data.split_off(SurbAck::len(Some(PacketType::Mix)));
            (hop_data, message)
        }

        /// Sends the SURB-ACK of a regular packet through every hop and returns its destination
        /// and the ack content that the gateway passes to the client.
        pub(crate) fn unwrap_surb_ack(
            &self,
            ack_data: &[u8],
        ) -> (DestinationAddressBytes, Vec<u8>) {
            let (_, ack_packet) =
                SurbAck::try_recover_first_hop_packet(ack_data, PacketType::Mix).unwrap();
            self.unwrap_all_hops(ack_packet)
        }
    }

    fn loop_stream(
        network: &TestNetwork,
        topology_access: TopologyAccessor,
        mix_tx: BatchMixMessageSender,
    ) -> LoopCoverTrafficStream<OsRng> {
        LoopCoverTrafficStream::new(
            network.ack_key.clone(),
            Duration::ZERO,
            mix_tx,
            network.address,
            topology_access,
            config::Traffic::default(),
            config::CoverTraffic::default(),
            ClientStatsSender::new(None, ShutdownToken::new()),
        )
    }

    async fn next_loop_cover_packet(network: &TestNetwork) -> MixPacket {
        let (mix_tx, mut mix_rx) = tokio::sync::mpsc::channel(1);
        let mut stream = loop_stream(network, network.topology_accessor(), mix_tx);
        stream.on_new_message().await;
        mix_rx.try_recv().unwrap().pop().unwrap()
    }

    // r[verify cover.loop-own-address]
    // r[verify cover.same-format]
    #[tokio::test]
    async fn loop_cover_packet_is_a_regular_packet_addressed_to_ourselves() {
        let network = TestNetwork::new();
        let packet = next_loop_cover_packet(&network).await;
        assert_eq!(packet.packet().len(), PacketSize::RegularPacket.size());

        let (destination, hop_data) = network.unwrap_all_hops(packet.into_packet());
        assert_eq!(destination, network.address.as_sphinx_destination().address);

        let (_, mut message) = TestNetwork::split_surb_ack(hop_data);
        let plaintext = SphinxMessageReceiver::new()
            .recover_plaintext_from_regular_packet(network.client_keys.private_key(), &mut message)
            .unwrap();
        let (prefix, rest) = plaintext.split_at(LOOP_COVER_MESSAGE_PAYLOAD.len());
        assert_eq!(prefix, LOOP_COVER_MESSAGE_PAYLOAD);
        assert_eq!(rest[0], 1);
        assert!(rest[1..].iter().all(|byte| *byte == 0));
    }

    // r[verify cover.loop-own-address]
    #[tokio::test]
    async fn loop_cover_surb_ack_returns_to_ourselves_with_the_cover_fragment_id() {
        let network = TestNetwork::new();
        let packet = next_loop_cover_packet(&network).await;
        let (_, hop_data) = network.unwrap_all_hops(packet.into_packet());
        let (ack_data, _) = TestNetwork::split_surb_ack(hop_data);

        let (ack_destination, ack_content) = network.unwrap_surb_ack(&ack_data);
        assert_eq!(
            ack_destination,
            network.address.as_sphinx_destination().address
        );
        assert_eq!(
            recover_identifier(&network.ack_key, &ack_content),
            Some(COVER_FRAG_ID.to_bytes())
        );
    }

    // r[verify cover.rate-can-drop]
    #[tokio::test]
    async fn loop_cover_stream_skips_its_packet_when_the_topology_is_invalid() {
        let network = TestNetwork::new();
        let (mix_tx, mut mix_rx) = tokio::sync::mpsc::channel(1);
        let mut stream = loop_stream(&network, TopologyAccessor::new(false), mix_tx);

        stream.on_new_message().await;
        assert!(mix_rx.try_recv().is_err());
    }

    // r[verify cover.rate-can-drop]
    #[tokio::test]
    async fn loop_cover_stream_drops_its_packet_when_the_gateway_channel_is_full() {
        let network = TestNetwork::new();
        let (mix_tx, mut mix_rx) = tokio::sync::mpsc::channel(1);
        mix_tx.try_send(Vec::new()).unwrap();
        let mut stream = loop_stream(&network, network.topology_accessor(), mix_tx);

        // a send that waits for space would never finish, because nothing reads the channel
        tokio::time::timeout(Duration::from_secs(5), stream.on_new_message())
            .await
            .expect("the loop cover stream waited for space in a full channel");
        assert!(mix_rx.try_recv().unwrap().is_empty());
        assert!(mix_rx.try_recv().is_err());
    }
}
