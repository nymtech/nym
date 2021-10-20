// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::receiver::{GatewayClientUpdate, GatewayClientUpdateSender};
use crypto::asymmetric::identity::{self, PUBLIC_KEY_LENGTH};
use futures::channel::mpsc;
use futures::stream::{self, FuturesUnordered, StreamExt};
use futures::task::Context;
use futures::{Future, Stream};
use gateway_client::error::GatewayClientError;
use gateway_client::{AcknowledgementReceiver, GatewayClient, MixnetMessageReceiver};
use log::{debug, info, warn};
use nymsphinx::forwarding::packet::MixPacket;
use pin_project::pin_project;
use std::collections::HashMap;
use std::mem;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tokio::time::Instant;

#[cfg(feature = "coconut")]
use coconut_interface::Credential;

const TIME_CHUNK_SIZE: Duration = Duration::from_millis(50);

pub(crate) struct GatewayPackets {
    /// Network address of the target gateway if wanted to be accessed by the client.
    /// It is a websocket address.
    clients_address: String,

    /// Public key of the target gateway.
    pub_key: identity::PublicKey,

    /// All the packets that are going to get sent to the gateway.
    packets: Vec<MixPacket>,
}

impl GatewayPackets {
    pub(crate) fn new(
        clients_address: String,
        pub_key: identity::PublicKey,
        packets: Vec<MixPacket>,
    ) -> Self {
        GatewayPackets {
            clients_address,
            pub_key,
            packets,
        }
    }

    pub(super) fn push_packets(&mut self, mut packets: Vec<MixPacket>) {
        self.packets.append(&mut packets)
    }

    pub(super) fn gateway_address(&self) -> identity::PublicKey {
        self.pub_key
    }
}

// struct consisting of all external data required to construct a fresh gateway client
struct FreshGatewayClientData {
    gateways_status_updater: GatewayClientUpdateSender,
    local_identity: Arc<identity::KeyPair>,
    gateway_response_timeout: Duration,

    // I guess in the future this struct will require aggregated verification key and....
    // ... something for obtaining actual credential

    // TODO:
    // SECURITY:
    // since currently we have no double spending protection, just to get things running
    // we're re-using the same credential for all gateways all the time. THIS IS VERY BAD!!
    #[cfg(feature = "coconut")]
    coconut_bandwidth_credential: Credential,
}

pub(crate) struct PacketSender {
    // TODO: this has a potential long-term issue. If we keep those clients cached between runs,
    // malicious gateways could figure out which traffic comes from the network monitor and always
    // forward that traffic while dropping the rest. However, at the current stage such sophisticated
    // behaviour is unlikely.
    active_gateway_clients: HashMap<[u8; PUBLIC_KEY_LENGTH], GatewayClient>,

    // I guess that will be required later on if credentials are got per gateway
    // aggregated_verification_key: Arc<VerificationKey>,
    fresh_gateway_client_data: Arc<FreshGatewayClientData>,
    gateway_connection_timeout: Duration,
    max_concurrent_clients: usize,
    max_sending_rate: usize,
}

impl PacketSender {
    pub(crate) fn new(
        gateways_status_updater: GatewayClientUpdateSender,
        local_identity: Arc<identity::KeyPair>,
        gateway_response_timeout: Duration,
        gateway_connection_timeout: Duration,
        max_concurrent_clients: usize,
        max_sending_rate: usize,
        #[cfg(feature = "coconut")] coconut_bandwidth_credential: Credential,
    ) -> Self {
        PacketSender {
            active_gateway_clients: HashMap::new(),
            fresh_gateway_client_data: Arc::new(FreshGatewayClientData {
                gateways_status_updater,
                local_identity,
                gateway_response_timeout,
                #[cfg(feature = "coconut")]
                coconut_bandwidth_credential,
            }),
            gateway_connection_timeout,
            max_concurrent_clients,
            max_sending_rate,
        }
    }

    async fn new_gateway_client(
        address: String,
        identity: identity::PublicKey,
        fresh_gateway_client_data: &FreshGatewayClientData,
    ) -> (
        GatewayClient,
        (MixnetMessageReceiver, AcknowledgementReceiver),
    ) {
        // TODO: future optimization: if we're remaking client for a gateway to which we used to be connected in the past,
        // use old shared keys
        let (message_sender, message_receiver) = mpsc::unbounded();

        // currently we do not care about acks at all, but we must keep the channel alive
        // so that the gateway client would not crash
        let (ack_sender, ack_receiver) = mpsc::unbounded();
        (
            GatewayClient::new(
                address,
                Arc::clone(&fresh_gateway_client_data.local_identity),
                identity,
                None,
                message_sender,
                ack_sender,
                fresh_gateway_client_data.gateway_response_timeout,
            ),
            (message_receiver, ack_receiver),
        )
    }

    async fn attempt_to_send_packets(
        client: &mut GatewayClient,
        mut mix_packets: Vec<MixPacket>,
        max_sending_rate: usize,
    ) -> Result<(), GatewayClientError> {
        let gateway_id = client.gateway_identity().to_base58_string();
        info!(
            target: "MessageSender",
            "Got {} packets to send to gateway {}",
            mix_packets.len(),
            gateway_id
        );

        if mix_packets.len() <= max_sending_rate {
            debug!(target: "MessageSender","Everything is going to get sent as one.");
            client.batch_send_mix_packets(mix_packets).await?;
        } else {
            let packets_per_time_chunk =
                (max_sending_rate as f64 * TIME_CHUNK_SIZE.as_secs_f64()) as usize;

            let total_expected_time =
                Duration::from_secs_f64(mix_packets.len() as f64 / max_sending_rate as f64);
            info!(
                target: "MessageSender",
                "With our rate of {} packets/s it should take around {:?} to send it all to {} ...",
                max_sending_rate, total_expected_time, gateway_id
            );

            fn split_off_vec(vec: &mut Vec<MixPacket>, at: usize) -> Option<Vec<MixPacket>> {
                if vec.is_empty() {
                    None
                } else {
                    if at >= vec.len() {
                        return Some(Vec::new());
                    }
                    Some(vec.split_off(at))
                }
            }

            // TODO future consideration: perhaps allow gateway client to take the packets by reference?
            // this way we won't have to do reallocations in here as they're unavoidable when
            // splitting a vector into multiple vectors
            while let Some(retained) = split_off_vec(&mut mix_packets, packets_per_time_chunk) {
                debug!(target: "MessageSender","Sending {} packets...", mix_packets.len());

                if mix_packets.len() == 1 {
                    client.send_mix_packet(mix_packets.pop().unwrap()).await?;
                } else {
                    client.batch_send_mix_packets(mix_packets).await?;
                }

                tokio::time::sleep(TIME_CHUNK_SIZE).await;

                mix_packets = retained;
            }
            debug!(target: "MessageSender", "Done sending");
        }

        Ok(())
    }

    // TODO: perhaps it should be spawned as a task to execute it in parallel rather
    // than just concurrently?
    async fn send_gateway_packets(
        gateway_connection_timeout: Duration,
        packets: GatewayPackets,
        fresh_gateway_client_data: Arc<FreshGatewayClientData>,
        client: Option<GatewayClient>,
        max_sending_rate: usize,
    ) -> Option<GatewayClient> {
        let was_present = client.is_some();

        let (mut client, gateway_channels) = if let Some(client) = client {
            (client, None)
        } else {
            let (mut new_client, (message_receiver, ack_receiver)) = Self::new_gateway_client(
                packets.clients_address,
                packets.pub_key,
                &fresh_gateway_client_data,
            )
            .await;

            // Put this in timeout in case the gateway has incorrectly set their ulimit and our connection
            // gets stuck in their TCP queue and just hangs on our end but does not terminate
            // (an actual bug we experienced)
            match tokio::time::timeout(
                gateway_connection_timeout,
                new_client.authenticate_and_start(
                    #[cfg(feature = "coconut")]
                    Some(
                        fresh_gateway_client_data
                            .coconut_bandwidth_credential
                            .clone(),
                    ),
                ),
            )
            .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(err)) => {
                    warn!(
                        "failed to authenticate with new gateway ({}) - {}",
                        packets.pub_key.to_base58_string(),
                        err
                    );
                    // we failed to create a client, can't do much here
                    return None;
                }
                Err(_) => {
                    warn!(
                        "timed out while trying to authenticate with new gateway ({})",
                        packets.pub_key.to_base58_string()
                    );
                    return None;
                }
            };

            (new_client, Some((message_receiver, ack_receiver)))
        };

        if let Err(err) =
            Self::attempt_to_send_packets(&mut client, packets.packets, max_sending_rate).await
        {
            warn!(
                "failed to send packets to {} - {:?}",
                packets.pub_key.to_base58_string(),
                err
            );
            // if this was a fresh client, there's no need to do anything as it was never
            // registered to get read
            if was_present {
                fresh_gateway_client_data
                    .gateways_status_updater
                    .unbounded_send(GatewayClientUpdate::Failure(packets.pub_key))
                    .expect("packet receiver seems to have died!");
            }
            return None;
        } else if !was_present {
            // this is a fresh and working client
            fresh_gateway_client_data
                .gateways_status_updater
                .unbounded_send(GatewayClientUpdate::New(
                    packets.pub_key,
                    gateway_channels
                        .expect("we created a new client, yet the channels are a None!"),
                ))
                .expect("packet receiver seems to have died!")
        }
        Some(client)
    }

    pub(super) async fn send_packets(&mut self, packets: Vec<GatewayPackets>) {
        // we know that each of the elements in the packets array will only ever access a single,
        // unique element from the existing clients

        // while it may seem weird that each time we send packets we remove the entries from the map,
        // and then put them back in, this way we remove the need for having locks instead, like
        // Arc<RwLock<HashMap<key, Mutex<GatewayClient>>>>
        let gateway_connection_timeout = self.gateway_connection_timeout;
        let max_concurrent_clients = if self.max_concurrent_clients > 0 {
            Some(self.max_concurrent_clients)
        } else {
            None
        };
        let max_sending_rate = self.max_sending_rate;

        // can't chain it all nicely together as there's no adapter method defined on Stream directly
        // for ForEachConcurrentClientUse
        let stream = stream::iter(packets.into_iter().map(|packets| {
            let existing_client = self
                .active_gateway_clients
                .remove(&(packets.pub_key.to_bytes()));
            (
                packets,
                Arc::clone(&self.fresh_gateway_client_data),
                existing_client,
            )
        }));

        ForEachConcurrentClientUse::new(
            stream,
            max_concurrent_clients,
            |(packets, fresh_data, client)| async move {
                Self::send_gateway_packets(
                    gateway_connection_timeout,
                    packets,
                    fresh_data,
                    client,
                    max_sending_rate,
                )
                .await
            },
        )
        .await
        .into_iter()
        .for_each(|client| {
            if let Some(client) = client {
                if let Some(existing) = self
                    .active_gateway_clients
                    .insert(client.gateway_identity().to_bytes(), client)
                {
                    panic!(
                        "we got duplicate gateway client for {}!",
                        existing.gateway_identity().to_base58_string()
                    );
                }
            }
        })
    }

    pub(super) async fn ping_all_active_gateways(&mut self) {
        if self.active_gateway_clients.is_empty() {
            info!(target: "Monitor", "no gateways to ping");
            return;
        }

        let ping_start = Instant::now();

        let mut clients_to_purge = Vec::new();

        // since we don't need to wait for response, we can just ping all gateways sequentially
        // if it becomes problem later on, we can adjust it.
        for (gateway_id, active_client) in self.active_gateway_clients.iter_mut() {
            if let Err(err) = active_client.send_ping_message().await {
                warn!(
                    target: "Monitor",
                    "failed to send ping message to gateway {} - {} - assuming the connection is dead.",
                    active_client.gateway_identity().to_base58_string(),
                    err,
                );
                clients_to_purge.push(*gateway_id);
            }
        }

        // purge all dead connections
        for gateway_id in clients_to_purge.into_iter() {
            // if this unwrap failed it means something extremely weird is going on
            // and we got some solar flare bitflip type of corruption
            let gateway_key = identity::PublicKey::from_bytes(&gateway_id)
                .expect("failed to recover gateways public key from valid bytes");

            // remove the gateway listener channels
            self.fresh_gateway_client_data
                .gateways_status_updater
                .unbounded_send(GatewayClientUpdate::Failure(gateway_key))
                .expect("packet receiver seems to have died!");

            // and remove it from our cache
            self.active_gateway_clients.remove(&gateway_id);
        }

        let ping_end = Instant::now();
        let time_taken = ping_end.duration_since(ping_start);
        debug!(target: "Monitor", "pinging all active gateways took {:?}", time_taken);
    }
}

// A slightly modified and less generic version of the futures' ForEachConcurrent that allows the futures to return
// gateway clients back
#[pin_project]
struct ForEachConcurrentClientUse<St, Fut, F> {
    #[pin]
    stream: Option<St>,
    f: F,
    futures: FuturesUnordered<Fut>,
    limit: Option<NonZeroUsize>,
    result: Vec<Option<GatewayClient>>,
}

impl<St, Fut, F> ForEachConcurrentClientUse<St, Fut, F>
where
    St: Stream,
    F: FnMut(St::Item) -> Fut,
    Fut: Future<Output = Option<GatewayClient>>,
{
    pub(super) fn new(stream: St, limit: Option<usize>, f: F) -> Self {
        let size_hint = stream.size_hint();
        Self {
            stream: Some(stream),
            // Note: `limit` = 0 gets ignored.
            limit: limit.and_then(NonZeroUsize::new),
            f,
            futures: FuturesUnordered::new(),
            result: Vec::with_capacity(size_hint.1.unwrap_or(size_hint.0)),
        }
    }
}

impl<St, Fut, F> Future for ForEachConcurrentClientUse<St, Fut, F>
where
    St: Stream,
    F: FnMut(St::Item) -> Fut,
    Fut: Future<Output = Option<GatewayClient>>,
{
    type Output = Vec<Option<GatewayClient>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let mut made_progress_this_iter = false;

            // Check if we've already created a number of futures greater than `limit`
            if this
                .limit
                .map(|limit| limit.get() > this.futures.len())
                .unwrap_or(true)
            {
                let mut stream_completed = false;
                let elem = if let Some(stream) = this.stream.as_mut().as_pin_mut() {
                    match stream.poll_next(cx) {
                        Poll::Ready(Some(elem)) => {
                            made_progress_this_iter = true;
                            Some(elem)
                        }
                        Poll::Ready(None) => {
                            stream_completed = true;
                            None
                        }
                        Poll::Pending => None,
                    }
                } else {
                    None
                };
                if stream_completed {
                    this.stream.set(None);
                }
                if let Some(elem) = elem {
                    this.futures.push((this.f)(elem));
                }
            }

            match this.futures.poll_next_unpin(cx) {
                Poll::Ready(Some(client)) => {
                    this.result.push(client);
                    made_progress_this_iter = true
                }
                Poll::Ready(None) => {
                    if this.stream.is_none() {
                        return Poll::Ready(mem::take(this.result));
                    }
                }
                Poll::Pending => {}
            }

            if !made_progress_this_iter {
                return Poll::Pending;
            }
        }
    }
}
