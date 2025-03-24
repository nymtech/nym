// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use futures::{stream, SinkExt, Stream, StreamExt};
use nym_crypto::asymmetric::x25519;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_framing::codec::{NymCodec, NymCodecError};
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_params::{PacketSize, PacketType};
use nym_sphinx_routing::generate_hop_delays;
use nym_sphinx_types::{
    Destination, DestinationAddressBytes, Node, NymPacket, DESTINATION_ADDRESS_LENGTH,
    IDENTIFIER_LENGTH,
};
use nym_task::ShutdownToken;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::time::{interval, sleep, Instant};
use tokio_util::codec::Framed;

pub(crate) struct ThroughputTestingClient {
    initial_sending_delay: Duration,
    current_batch_size: usize,
    forward_packet_bytes: Vec<u8>,
    shutdown_token: ShutdownToken,
    local_address: SocketAddr,
    listener: TcpListener,
    forward_connection: Framed<TcpStream, NymCodec>,
}

impl ThroughputTestingClient {
    pub(crate) async fn try_create(
        initial_sending_delay: Duration,
        initial_batch_size: usize,
        node_key: x25519::PublicKey,
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
                node_key.into(),
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
            initial_sending_delay,
            current_batch_size: initial_batch_size,
            forward_packet_bytes: forward_packet.to_bytes()?,
            shutdown_token: cancellation_token,
            local_address,
            listener,
            forward_connection: Framed::new(forward_connection, NymCodec),
        })
    }

    async fn send_packets(&mut self) -> anyhow::Result<()> {
        println!("sending");
        let mut batch = Vec::with_capacity(self.current_batch_size);
        for _ in 0..self.current_batch_size {
            // that's a hacky 'clone', but properly doing it would have required updating the sphinx packet lib
            let forward_packet = NymPacket::sphinx_from_bytes(&self.forward_packet_bytes)?;
            let framed_packet = FramedNymPacket::new(forward_packet, PacketType::default());
            batch.push(Ok(framed_packet));
        }

        self.forward_connection
            .send_all(&mut stream::iter(batch))
            .await?;
        Ok(())
    }

    // don't bother processing packets, just increment the count because that's the only thing that matters
    fn handle_received(&mut self, maybe_packet: Result<FramedNymPacket, NymCodecError>) {
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
                    println!("received: {:?}", ingress_connection.inner.is_some());
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
