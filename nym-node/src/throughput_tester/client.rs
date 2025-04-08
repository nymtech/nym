// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::throughput_tester::stats::ClientStats;
use anyhow::bail;
use arrayref::array_ref;
use blake2::VarBlake2b;
use chacha::ChaCha;
use futures::{stream, SinkExt, Stream, StreamExt};
use hkdf::Hkdf;
use human_repr::{HumanCount, HumanDuration, HumanThroughput};
use lioness::Lioness;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_framing::codec::{NymCodec, NymCodecError};
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_params::PacketSize;
use nym_sphinx_routing::generate_hop_delays;
use nym_sphinx_types::constants::{
    EXPANDED_SHARED_SECRET_HKDF_INFO, EXPANDED_SHARED_SECRET_HKDF_SALT,
    EXPANDED_SHARED_SECRET_LENGTH,
};
use nym_sphinx_types::{
    Destination, DestinationAddressBytes, Node, NymPacket, PayloadKey, DESTINATION_ADDRESS_LENGTH,
    IDENTIFIER_LENGTH,
};
use nym_task::ShutdownToken;
use rand::rngs::OsRng;
use sha2::Sha256;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::time::{interval, sleep, Instant};
use tokio_util::codec::Framed;
use tracing::{debug, error, info, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

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

    fn elapsed_nanos(&self) -> u64 {
        // here we're making few assumptions: the latency is lower than u64::MAX
        // and it's strictly positive (which are rather valid...)
        self.elapsed().whole_nanoseconds() as u64
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
    stats: ClientStats,
    last_received_update: Instant,
    last_received_at_update: usize,
    current_batch: u64,
    sending_delay: Duration,
    latency_threshold: Duration,
    current_batch_size: usize,
    forward_header_bytes: Vec<u8>,
    unwrapped_forward_payload_bytes: Vec<u8>,
    shutdown_token: ShutdownToken,
    local_address: SocketAddr,
    listener: TcpListener,
    forward_connection: Framed<TcpStream, NymCodec>,
    payload_key: PayloadKey,
}

fn rederive_lioness_payload_key(shared_secret: &[u8; 32]) -> PayloadKey {
    let hkdf = Hkdf::<Sha256>::new(Some(EXPANDED_SHARED_SECRET_HKDF_SALT), shared_secret);

    // expanded shared secret
    let mut output = [0u8; EXPANDED_SHARED_SECRET_LENGTH];
    // SAFETY: the length of the provided okm is within the allowed range
    #[allow(clippy::unwrap_used)]
    hkdf.expand(EXPANDED_SHARED_SECRET_HKDF_INFO, &mut output)
        .unwrap();

    *array_ref!(&output, 32, 192)
}

impl ThroughputTestingClient {
    pub(crate) async fn try_create(
        initial_sending_delay: Duration,
        initial_batch_size: usize,
        latency_threshold: Duration,
        node_keys: &x25519::KeyPair,
        node_listener: SocketAddr,
        stats: ClientStats,
        cancellation_token: ShutdownToken,
    ) -> anyhow::Result<Self> {
        // attempt to bind to some port to receive processed packets
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
        let local_address = listener.local_addr()?;

        info!("listening on {local_address}");

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
        let sphinx_packet = forward_packet.to_sphinx_packet().unwrap();
        let header = &sphinx_packet.header;

        // derive the expanded shared secret for our node so we could tag the payload to figure out latency
        // by tagging the packet
        let shared_secret = node_keys
            .private_key()
            .as_ref()
            .diffie_hellman(&header.shared_secret);
        let payload_key = rederive_lioness_payload_key(shared_secret.as_bytes());

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
            stats,
            last_received_update: Instant::now(),
            last_received_at_update: 0,
            current_batch: 0,
            sending_delay: initial_sending_delay,
            latency_threshold,
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

    fn update_progress_bar(&mut self) {
        let received = self.stats.received();
        let sent = self.stats.sent();
        let latency = self.stats.average_latency_duration();
        let received_since_update = received - self.last_received_at_update;

        let time_delta_secs = self.last_received_update.elapsed().as_secs_f64();
        let receive_rate = received_since_update as f64 / time_delta_secs;

        self.last_received_at_update = received;
        self.last_received_update = Instant::now();
        // I couldn't figure out how to directly pull it from span fields without duplication,
        // so that's a second best
        Span::current().pb_set_message(&format!(
            "{}: CURRENT SENDING DELAY/BATCH: {} / {} | received: {} sent: {} (avg packet latency: {}, avg receive rate: {})",
            self.local_address,
            self.sending_delay.human_duration(),
            self.current_batch_size,
            received.human_count_bare(),
            sent.human_count_bare(),
            latency.human_duration(),
            receive_rate.human_throughput("packets")
        ));
    }

    fn lioness_encrypt(&self, block: &mut [u8]) -> anyhow::Result<()> {
        let lioness_cipher = Lioness::<VarBlake2b, ChaCha>::new_raw(&self.payload_key);
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
        self.stats.new_sent_batch(self.current_batch_size);

        Ok(())
    }

    // don't bother processing packets, just increment the count because that's the only thing that matters
    fn handle_received(&mut self, maybe_packet: Result<FramedNymPacket, NymCodecError>) {
        let Ok(received) = maybe_packet else {
            error!("FAILED TO RECEIVE PACKET");
            return;
        };
        let inner = received.into_inner();
        // safety: we sent a sphinx packet...
        #[allow(clippy::unwrap_used)]
        let sphinx = inner.to_sphinx_packet().unwrap();
        let tag = PacketTag::from_bytes(sphinx.payload.as_bytes());

        self.stats.new_received(tag.elapsed_nanos());
    }

    fn update_sending_rates(&mut self) {
        let current = self.stats.average_latency_nanos() as f64;
        let threshold = self.latency_threshold.as_nanos() as f64;

        let saturation = current / threshold;

        let sending_delay_nanos = self.sending_delay.as_nanos();
        let batch_size = self.current_batch_size;

        let diff = 1. - saturation;

        if saturation > 1. {
            debug!("saturation {saturation:.2}, packet latency over threshold: need to decrease sending rate");
        } else {
            debug!("saturation {saturation:.2}, packet latency under threshold: can increase sending rate");
        }

        // be conservative and only apply 50% of the diff
        // (and split it equally between sending delay and batch size)
        // but also make sure the current values don't increase by more than 5%
        let mut new_batch_size = (batch_size as f64 * (1. + 0.25 * diff)).floor() as u64;
        let mut new_sending_delay_nanos =
            (sending_delay_nanos as f64 * (1. - 0.25 * diff)).floor() as u64;

        if (new_batch_size as f64) > (batch_size as f64 * 1.05) {
            new_batch_size = ((batch_size as f64) * 1.05) as u64;
        }
        if (new_batch_size as f64) < (batch_size as f64 * 0.95) {
            new_batch_size = ((batch_size as f64) * 0.95) as u64;
        }

        if (new_sending_delay_nanos as f64) > (sending_delay_nanos as f64 * 1.05) {
            new_sending_delay_nanos = ((sending_delay_nanos as f64) * 1.05) as u64;
        }
        if (new_sending_delay_nanos as f64) < (sending_delay_nanos as f64 * 0.95) {
            new_sending_delay_nanos = ((sending_delay_nanos as f64) * 0.95) as u64;
        }

        // normalize values
        if new_batch_size < 20 {
            new_batch_size = 20;
        }
        let mut new_sending_delay = Duration::from_nanos(new_sending_delay_nanos);

        if new_sending_delay.is_zero() {
            new_sending_delay = Duration::from_micros(500);
        }
        if new_sending_delay.as_millis() > 100 {
            new_sending_delay = Duration::from_millis(100);
        }

        debug!(
            "changing sending delay from {} to {}",
            self.sending_delay.human_duration(),
            new_sending_delay.human_duration()
        );
        debug!("changing sending batch from {batch_size} to {new_batch_size}");

        self.sending_delay = new_sending_delay;
        self.current_batch_size = new_batch_size as usize;
    }

    #[allow(clippy::panic)]
    pub(crate) async fn run(mut self) -> anyhow::Result<()> {
        let mut ingress_connection = StreamWrapper::default();

        let mut sending_interval = interval(self.sending_delay);
        sending_interval.reset();

        // quite arbitrary
        let mut update_interval = interval(Duration::from_millis(500));
        update_interval.reset();

        let mut last_rate_update = Instant::now();

        loop {
            select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    info!("cancelled");
                    return Ok(());
                }
                _ = update_interval.tick() => {
                    self.update_progress_bar();

                    // every 500ms attempt to adjust sending rates
                    if last_rate_update.elapsed() > Duration::from_millis(500) {
                        last_rate_update = Instant::now();
                        self.update_sending_rates();
                        sending_interval = interval(self.sending_delay);
                        sending_interval.reset();
                    }

                }
                accepted = self.listener.accept() => {
                    info!("accepted connection");
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
                 _ = sending_interval.tick() => {
                    self.send_packets().await?;
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
