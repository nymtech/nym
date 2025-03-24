// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use arrayref::array_ref;
use blake2::VarBlake2b;
use chacha::ChaCha;
use futures::{stream, SinkExt, Stream, StreamExt};
use lioness::Lioness;
use nym_crypto::asymmetric::x25519;
use nym_pemstore::traits::PemStorableKeyPair;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_framing::codec::{NymCodec, NymCodecError};
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_params::{PacketSize, PacketType};
use nym_sphinx_routing::generate_hop_delays;
use nym_sphinx_types::header::keys::PayloadKey;
use nym_sphinx_types::{
    Destination, DestinationAddressBytes, Node, NymPacket, SphinxHeader,
    DESTINATION_ADDRESS_LENGTH, HEADER_SIZE, IDENTIFIER_LENGTH,
};
use nym_task::ShutdownToken;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::time::{interval, sleep, Instant};
use tokio_util::codec::Framed;

struct PacketTag {
    sending_timestamp: OffsetDateTime,
    batch_id: u64,
    index: u64,
}

impl PacketTag {
    const SIZE: usize = 32;

    fn elapsed(&self) -> time::Duration {
        OffsetDateTime::now_utc() - self.sending_timestamp
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.sending_timestamp
            .unix_timestamp_nanos()
            .to_be_bytes()
            .into_iter()
            .chain(self.batch_id.to_be_bytes())
            .chain(self.index.to_be_bytes())
            .collect()
    }

    #[allow(clippy::unwrap_used)]
    fn from_bytes(bytes: &[u8]) -> PacketTag {
        let sending_timestamp = i128::from_be_bytes(bytes[0..16].try_into().unwrap());
        let sending_timestamp =
            OffsetDateTime::from_unix_timestamp_nanos(sending_timestamp).unwrap();

        let batch_id = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let index = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
        PacketTag {
            sending_timestamp,
            batch_id,
            index,
        }
    }
}

pub(crate) struct ThroughputTestingClient {
    current_batch: u64,
    initial_sending_delay: Duration,
    current_batch_size: usize,
    forward_header_bytes: Vec<u8>,
    unwrapped_forward_payload_bytes: Vec<u8>,
    shutdown_token: ShutdownToken,
    local_address: SocketAddr,
    listener: TcpListener,
    forward_connection: Framed<TcpStream, NymCodec>,
    payload_key: PayloadKey,
}

impl ThroughputTestingClient {
    pub(crate) async fn try_create(
        initial_sending_delay: Duration,
        initial_batch_size: usize,
        node_keys: &x25519::KeyPair,
        node_listener: SocketAddr,
        cancellation_token: ShutdownToken,
    ) -> anyhow::Result<Self> {
        // attempt to bind to some port to receive processed packets
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
        let local_address = listener.local_addr()?;

        println!("Listening on {}", local_address);

        // create the sphinx packet we're going to be repeatedly sending
        // (next hop has to be our mixnode, then this client, and then it doesn't matter since the packet won't
        // get further processed)
        let mut rng = OsRng;
        // keys of this client
        let ephemeral_keys = x25519::KeyPair::new(&mut rng);

        let route = [
            Node::new(
                NymNodeRoutingAddress::from(node_listener).try_into()?,
                (*node_keys.public_key()).into(),
            ),
            Node::new(
                NymNodeRoutingAddress::from(local_address).try_into()?,
                (*ephemeral_keys.public_key()).into(),
            ),
            Node::new(
                NymNodeRoutingAddress::from(local_address).try_into()?,
                (*ephemeral_keys.public_key()).into(),
            ),
        ];
        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([0u8; DESTINATION_ADDRESS_LENGTH]),
            [0u8; IDENTIFIER_LENGTH],
        );
        let delays = generate_hop_delays(Duration::default(), 3);
        let payload = PacketSize::RegularPacket.payload_size();

        let forward_packet =
            NymPacket::sphinx_build(payload, b"foomp", &route, &destination, &delays)?;

        // SAFETY: we constructed a sphinx packet...
        #[allow(clippy::unwrap_used)]
        let sphinx_packet = forward_packet.as_sphinx_packet().unwrap();
        let header = &sphinx_packet.header;

        // derive the routing keys of our node so we could tag the payload to figure out latency
        // by tagging the packet
        let routing_keys = SphinxHeader::compute_routing_keys(
            &header.shared_secret,
            (&node_keys.private_key()).as_ref(),
        );
        let payload_key = routing_keys.payload_key;
        let unwrapped_payload = sphinx_packet.payload.unwrap(&payload_key)?;
        let unwrapped_forward_payload_bytes = unwrapped_payload.into_bytes();

        let start = Instant::now();
        let forward_connection = loop {
            if let Ok(connection) = TcpStream::connect(node_listener).await {
                break connection;
            }
            // fallback
            sleep(Duration::from_secs(1)).await;
            if start.elapsed() > Duration::from_secs(10) {
                bail!("failed to connect to local nym-node")
            }
        };

        Ok(ThroughputTestingClient {
            current_batch: 0,
            initial_sending_delay,
            current_batch_size: initial_batch_size,
            forward_header_bytes: sphinx_packet.header.to_bytes(),
            unwrapped_forward_payload_bytes,
            shutdown_token: cancellation_token,
            local_address,
            listener,
            forward_connection: Framed::new(forward_connection, NymCodec),
            payload_key,
        })
    }

    fn lioness_encrypt(&self, block: &mut [u8]) -> anyhow::Result<()> {
        let lioness_cipher = Lioness::<VarBlake2b, ChaCha>::new_raw(array_ref!(
            self.payload_key,
            0,
            lioness::RAW_KEY_SIZE
        ));
        lioness_cipher.encrypt(block)?;
        Ok(())
    }

    fn tag_framed_packet(&self, tag: PacketTag) -> anyhow::Result<FramedNymPacket> {
        let tag_bytes = tag.to_bytes();

        let mut payload_bytes = self.unwrapped_forward_payload_bytes.clone();
        payload_bytes[..PacketTag::SIZE].copy_from_slice(&tag_bytes);

        self.lioness_encrypt(&mut payload_bytes)?;

        let mut packet_bytes = self.forward_header_bytes.clone();
        packet_bytes.append(&mut payload_bytes);

        let forward_packet = NymPacket::sphinx_from_bytes(&packet_bytes)?;
        Ok(FramedNymPacket::new(forward_packet, Default::default()))
    }

    async fn send_packets(&mut self) -> anyhow::Result<()> {
        // mess with our payload in such a way that upon unwrapping by the first hop,
        // we'll get our tag

        println!("sending");
        let mut batch = Vec::with_capacity(self.current_batch_size);
        let now = OffsetDateTime::now_utc();
        for i in 0..self.current_batch_size {
            let tag = PacketTag {
                sending_timestamp: now,
                batch_id: self.current_batch,
                index: i as u64,
            };
            let framed_packet = self.tag_framed_packet(tag)?;
            batch.push(Ok(framed_packet));
        }

        self.current_batch += 1;
        self.forward_connection
            .send_all(&mut stream::iter(batch))
            .await?;
        Ok(())
    }

    // don't bother processing packets, just increment the count because that's the only thing that matters
    fn handle_received(&mut self, maybe_packet: Result<FramedNymPacket, NymCodecError>) {
        let Ok(received) = maybe_packet else {
            println!("FAILED TO RECEIVE PACKET");
            return;
        };
        let inner = received.into_inner();
        // safety: we sent a sphinx packet...
        #[allow(clippy::unwrap_used)]
        let sphinx = inner.as_sphinx_packet().unwrap();
        let tag = PacketTag::from_bytes(sphinx.payload.as_bytes());
        println!("packet latency: {:?}s", tag.elapsed().as_seconds_f32());

        // TODO: mess with sizing/tagging somehow
        println!("received packet - TODO: increase counters etc.")
    }

    // given we're running locally, we assume transmission delay is negligible
    // (in reality it's probably few ms, but it's a good enough approximation for now)

    #[allow(clippy::panic)]
    pub(crate) async fn run(mut self) -> anyhow::Result<()> {
        let mut ingress_connection = StreamWrapper::default();

        let mut sending_interval = interval(self.initial_sending_delay);
        sending_interval.reset();

        loop {
            select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    println!("cancelled");
                    return Ok(());
                }
                _ = sending_interval.tick() => {
                    println!("send");
                    self.send_packets().await?;
                }
                accepted = self.listener.accept() => {
                    println!("accept");
                    if ingress_connection.inner.is_some() {
                        // this should never happen under local settings
                        // (and since it's not exposed to 'proper' traffic, it's fine to panic and shutdown)
                        panic!("attempted to overwrite existing connection")
                    }
                    let (stream, _) = accepted?;
                    let framed = Framed::new(stream, NymCodec);
                    ingress_connection.set(framed);
                }
                received = ingress_connection.next() => {
                    let Some(received) = received else {
                        // if the stream has terminated, we return
                        if ingress_connection.inner.is_some() {
                            return Ok(())
                        }
                        continue;
                    };
                    self.handle_received(received)
                }
            }
        }
    }
}

// I must be blind, because I couldn't find something to do equivalent of `OptionStream`...
#[derive(Default)]
struct StreamWrapper {
    inner: Option<Framed<TcpStream, NymCodec>>,
    maybe_initial_waker: Option<Waker>,
}

impl StreamWrapper {
    fn set(&mut self, inner: Framed<TcpStream, NymCodec>) {
        self.inner = Some(inner);
        if let Some(waker) = self.maybe_initial_waker.take() {
            waker.wake();
        }
    }
}

impl Stream for StreamWrapper {
    type Item = <Framed<TcpStream, NymCodec> as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut() {
            None => {
                self.maybe_initial_waker = Some(cx.waker().clone());
                Poll::Pending
            }
            Some(inner) => Pin::new(inner).poll_next(cx),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            None => (0, None),
            Some(inner) => inner.size_hint(),
        }
    }
}
