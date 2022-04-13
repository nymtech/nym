// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::gateway_clients_cache::{
    ActiveGatewayClients, GatewayClientHandle,
};
use crate::network_monitor::monitor::gateways_pinger::GatewayPinger;
use crate::network_monitor::monitor::receiver::{GatewayClientUpdate, GatewayClientUpdateSender};
use config::defaults::REMAINING_BANDWIDTH_THRESHOLD;
use credential_storage::PersistentStorage;
use crypto::asymmetric::identity::{self, PUBLIC_KEY_LENGTH};
use futures::channel::mpsc;
use futures::stream::{self, FuturesUnordered, StreamExt};
use futures::task::Context;
use futures::{Future, Stream};
use gateway_client::error::GatewayClientError;
use gateway_client::{AcknowledgementReceiver, GatewayClient, MixnetMessageReceiver};
use log::{debug, info, trace, warn};
use nymsphinx::forwarding::packet::MixPacket;
use pin_project::pin_project;
use std::mem;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use gateway_client::bandwidth::BandwidthController;

const TIME_CHUNK_SIZE: Duration = Duration::from_millis(50);

pub(crate) struct GatewayPackets {
    /// Network address of the target gateway if wanted to be accessed by the client.
    /// It is a websocket address.
    pub(crate) clients_address: String,

    /// Public key of the target gateway.
    pub(crate) pub_key: identity::PublicKey,

    /// The address of the gateway owner.
    pub(crate) gateway_owner: String,

    /// All the packets that are going to get sent to the gateway.
    pub(crate) packets: Vec<MixPacket>,
}

impl GatewayPackets {
    pub(crate) fn new(
        clients_address: String,
        pub_key: identity::PublicKey,
        gateway_owner: String,
        packets: Vec<MixPacket>,
    ) -> Self {
        GatewayPackets {
            clients_address,
            pub_key,
            gateway_owner,
            packets,
        }
    }

    pub(crate) fn empty(
        clients_address: String,
        pub_key: identity::PublicKey,
        gateway_owner: String,
    ) -> Self {
        GatewayPackets {
            clients_address,
            pub_key,
            gateway_owner,
            packets: Vec::new(),
        }
    }

    pub(super) fn push_packets(&mut self, mut packets: Vec<MixPacket>) {
        if self.packets.is_empty() {
            self.packets = packets
        } else if self.packets.len() > packets.len() {
            self.packets.append(&mut packets)
        } else {
            packets.append(&mut self.packets);
            self.packets = packets;
        }
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
    // for coconut bandwidth credentials we currently have no double spending protection, just to
    // get things running we're re-using the same credential for all gateways all the time.
    // THIS IS VERY BAD!!
    bandwidth_controller: BandwidthController<PersistentStorage>,
    testnet_mode: bool,
}

impl FreshGatewayClientData {
    fn notify_connection_failure(
        self: Arc<FreshGatewayClientData>,
        raw_gateway_id: [u8; PUBLIC_KEY_LENGTH],
    ) {
        // if this unwrap failed it means something extremely weird is going on
        // and we got some solar flare bitflip type of corruption
        let gateway_key = identity::PublicKey::from_bytes(&raw_gateway_id)
            .expect("failed to recover gateways public key from valid bytes");

        // remove the gateway listener channels
        self.gateways_status_updater
            .unbounded_send(GatewayClientUpdate::Failure(gateway_key))
            .expect("packet receiver seems to have died!");
    }

    fn notify_new_connection(
        self: Arc<FreshGatewayClientData>,
        gateway_id: identity::PublicKey,
        gateway_channels: Option<(MixnetMessageReceiver, AcknowledgementReceiver)>,
    ) {
        self.gateways_status_updater
            .unbounded_send(GatewayClientUpdate::New(
                gateway_id,
                gateway_channels.expect("we created a new client, yet the channels are a None!"),
            ))
            .expect("packet receiver seems to have died!")
    }
}

pub(crate) struct PacketSender {
    // TODO: this has a potential long-term issue. If we keep those clients cached between runs,
    // malicious gateways could figure out which traffic comes from the network monitor and always
    // forward that traffic while dropping the rest. However, at the current stage such sophisticated
    // behaviour is unlikely.
    active_gateway_clients: ActiveGatewayClients,

    // I guess that will be required later on if credentials are got per gateway
    // aggregated_verification_key: Arc<VerificationKey>,
    fresh_gateway_client_data: Arc<FreshGatewayClientData>,
    gateway_connection_timeout: Duration,
    max_concurrent_clients: usize,
    max_sending_rate: usize,
}

impl PacketSender {
    // at this point I'm not entirely sure how to deal with this warning without
    // some considerable refactoring
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        gateways_status_updater: GatewayClientUpdateSender,
        local_identity: Arc<identity::KeyPair>,
        gateway_response_timeout: Duration,
        gateway_connection_timeout: Duration,
        max_concurrent_clients: usize,
        max_sending_rate: usize,
        bandwidth_controller: BandwidthController<PersistentStorage>,
        testnet_mode: bool,
    ) -> Self {
        PacketSender {
            active_gateway_clients: ActiveGatewayClients::new(),
            fresh_gateway_client_data: Arc::new(FreshGatewayClientData {
                gateways_status_updater,
                local_identity,
                gateway_response_timeout,
                bandwidth_controller,
                testnet_mode,
            }),
            gateway_connection_timeout,
            max_concurrent_clients,
            max_sending_rate,
        }
    }

    pub(crate) fn spawn_gateways_pinger(&self, pinging_interval: Duration) {
        let gateway_pinger = GatewayPinger::new(
            self.active_gateway_clients.clone(),
            self.fresh_gateway_client_data
                .gateways_status_updater
                .clone(),
            pinging_interval,
        );

        tokio::spawn(async move { gateway_pinger.run().await });
    }

    fn new_gateway_client_handle(
        address: String,
        identity: identity::PublicKey,
        owner: String,
        fresh_gateway_client_data: &FreshGatewayClientData,
    ) -> (
        GatewayClientHandle,
        (MixnetMessageReceiver, AcknowledgementReceiver),
    ) {
        // TODO: future optimization: if we're remaking client for a gateway to which we used to be connected in the past,
        // use old shared keys
        let (message_sender, message_receiver) = mpsc::unbounded();

        // currently we do not care about acks at all, but we must keep the channel alive
        // so that the gateway client would not crash
        let (ack_sender, ack_receiver) = mpsc::unbounded();
        let mut gateway_client = GatewayClient::new(
            address,
            Arc::clone(&fresh_gateway_client_data.local_identity),
            identity,
            owner,
            None,
            message_sender,
            ack_sender,
            fresh_gateway_client_data.gateway_response_timeout,
            Some(fresh_gateway_client_data.bandwidth_controller.clone()),
        );

        if fresh_gateway_client_data.testnet_mode {
            gateway_client.set_testnet_mode(true)
        }

        (
            GatewayClientHandle::new(gateway_client),
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
            "Got {} packets to send to gateway {}",
            mix_packets.len(),
            gateway_id
        );

        if mix_packets.len() <= max_sending_rate {
            debug!("Everything is going to get sent as one.");
            client.batch_send_mix_packets(mix_packets).await?;
        } else {
            let packets_per_time_chunk =
                (max_sending_rate as f64 * TIME_CHUNK_SIZE.as_secs_f64()) as usize;

            let total_expected_time =
                Duration::from_secs_f64(mix_packets.len() as f64 / max_sending_rate as f64);
            info!(
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
                trace!("Sending {} packets...", mix_packets.len());

                if mix_packets.len() == 1 {
                    client.send_mix_packet(mix_packets.pop().unwrap()).await?;
                } else {
                    client.batch_send_mix_packets(mix_packets).await?;
                }

                tokio::time::sleep(TIME_CHUNK_SIZE).await;

                mix_packets = retained;
            }
            debug!("Done sending");
        }

        Ok(())
    }

    async fn create_new_gateway_client_handle_and_authenticate(
        address: String,
        identity: identity::PublicKey,
        owner: String,
        fresh_gateway_client_data: &FreshGatewayClientData,
        gateway_connection_timeout: Duration,
    ) -> Option<(
        GatewayClientHandle,
        (MixnetMessageReceiver, AcknowledgementReceiver),
    )> {
        let (new_client, (message_receiver, ack_receiver)) =
            Self::new_gateway_client_handle(address, identity, owner, fresh_gateway_client_data);

        // Put this in timeout in case the gateway has incorrectly set their ulimit and our connection
        // gets stuck in their TCP queue and just hangs on our end but does not terminate
        // (an actual bug we experienced)
        //
        // Note: locking the client in unchecked manner is fine here as we just created the lock
        // and it wasn't shared with anyone, therefore we're the only one holding reference to it
        // and hence it's impossible to fail to obtain the permit.
        let mut unlocked_client = new_client.lock_client_unchecked();
        match tokio::time::timeout(
            gateway_connection_timeout,
            unlocked_client.get_mut_unchecked().authenticate_and_start(),
        )
        .await
        {
            Ok(Ok(_)) => {
                drop(unlocked_client);
                Some((new_client, (message_receiver, ack_receiver)))
            }
            Ok(Err(err)) => {
                warn!(
                    "failed to authenticate with new gateway ({}) - {}",
                    identity.to_base58_string(),
                    err
                );
                // we failed to create a client, can't do much here
                None
            }
            Err(_) => {
                warn!(
                    "timed out while trying to authenticate with new gateway ({})",
                    identity.to_base58_string()
                );
                None
            }
        }
    }

    async fn check_remaining_bandwidth(
        client: &mut GatewayClient,
    ) -> Result<(), GatewayClientError> {
        if client.remaining_bandwidth() < REMAINING_BANDWIDTH_THRESHOLD {
            Err(GatewayClientError::NotEnoughBandwidth(
                REMAINING_BANDWIDTH_THRESHOLD,
                client.remaining_bandwidth(),
            ))
        } else {
            Ok(())
        }
    }

    // TODO: perhaps it should be spawned as a task to execute it in parallel rather
    // than just concurrently?
    async fn send_gateway_packets(
        gateway_connection_timeout: Duration,
        packets: GatewayPackets,
        fresh_gateway_client_data: Arc<FreshGatewayClientData>,
        client: Option<GatewayClientHandle>,
        max_sending_rate: usize,
    ) -> Option<GatewayClientHandle> {
        let existing_client = client.is_some();

        // Note that in the worst case scenario we will only wait for a second or two to obtain the lock
        // as other possibly entity holding the lock (the gateway pinger) is attempting to send
        // the ping messages with a maximum timeout.
        let (client, gateway_channels) = if let Some(client) = client {
            if client.is_invalid().await {
                warn!("Our existing client was invalid - two test runs happened back to back without cleanup");
                return None;
            }
            (client, None)
        } else {
            let (client, gateway_channels) =
                Self::create_new_gateway_client_handle_and_authenticate(
                    packets.clients_address,
                    packets.pub_key,
                    packets.gateway_owner,
                    &fresh_gateway_client_data,
                    gateway_connection_timeout,
                )
                .await?;
            (client, Some(gateway_channels))
        };

        let estimated_time =
            Duration::from_secs_f64(packets.packets.len() as f64 / max_sending_rate as f64);
        // give some leeway
        let timeout = estimated_time * 3;

        let mut guard = client.lock_client().await;
        let unwrapped_client = guard.get_mut_unchecked();

        if let Err(err) = Self::check_remaining_bandwidth(unwrapped_client).await {
            warn!(
                "Failed to claim additional bandwidth for {} - {}",
                unwrapped_client.gateway_identity().to_base58_string(),
                err
            );
            if existing_client {
                guard.invalidate();
                fresh_gateway_client_data.notify_connection_failure(packets.pub_key.to_bytes());
            }
            return None;
        }

        match tokio::time::timeout(
            timeout,
            Self::attempt_to_send_packets(unwrapped_client, packets.packets, max_sending_rate),
        )
        .await
        {
            Err(_timeout) => {
                warn!(
                    "failed to send packets to {} - we timed out",
                    packets.pub_key.to_base58_string(),
                );
                // if this was a fresh client, there's no need to do anything as it was never
                // registered to get read
                if existing_client {
                    guard.invalidate();
                    fresh_gateway_client_data.notify_connection_failure(packets.pub_key.to_bytes());
                }
                return None;
            }
            Ok(Err(err)) => {
                warn!(
                    "failed to send packets to {} - {:?}",
                    packets.pub_key.to_base58_string(),
                    err
                );
                // if this was a fresh client, there's no need to do anything as it was never
                // registered to get read
                if existing_client {
                    guard.invalidate();
                    fresh_gateway_client_data.notify_connection_failure(packets.pub_key.to_bytes());
                }
                return None;
            }
            Ok(Ok(_)) => {
                if !existing_client {
                    fresh_gateway_client_data
                        .notify_new_connection(packets.pub_key, gateway_channels);
                }
            }
        }

        drop(guard);
        Some(client)
    }

    // point of this is to basically insert handles of fresh clients that didn't exist here before
    async fn merge_client_handles(&self, handles: Vec<GatewayClientHandle>) {
        let mut guard = self.active_gateway_clients.lock().await;
        for handle in handles {
            let raw_identity = handle.raw_identity();
            if let Some(existing) = guard.get(&raw_identity) {
                if !handle.ptr_eq(existing) {
                    panic!("Duplicate client detected!")
                }

                if handle.is_invalid().await {
                    guard.remove(&raw_identity);
                }
            } else {
                // client never existed -> just insert it
                guard.insert(raw_identity, handle);
            }
        }
    }

    pub(super) async fn send_packets(&mut self, packets: Vec<GatewayPackets>) {
        // we know that each of the elements in the packets array will only ever access a single,
        // unique element from the existing clients

        let gateway_connection_timeout = self.gateway_connection_timeout;
        let max_concurrent_clients = if self.max_concurrent_clients > 0 {
            Some(self.max_concurrent_clients)
        } else {
            None
        };
        let max_sending_rate = self.max_sending_rate;

        let guard = self.active_gateway_clients.lock().await;
        // this clippy warning is a false positive as we cannot get rid of the collect by moving
        // everything into a single iterator as it would require us to hold the lock the entire time
        // and that is exactly what we want to avoid
        #[allow(clippy::needless_collect)]
        let stream_data = packets
            .into_iter()
            .map(|packets| {
                let existing_client = guard
                    .get(&packets.pub_key.to_bytes())
                    .map(|client| client.clone_data_pointer());
                (
                    packets,
                    Arc::clone(&self.fresh_gateway_client_data),
                    existing_client,
                )
            })
            .collect::<Vec<_>>();

        // drop the guard immediately so that the other task (gateway pinger) would not need to wait until
        // we're done sending packets (note: without this drop, we wouldn't be able to ping gateways that
        // we're not interacting with right now)
        drop(guard);

        // can't chain it all nicely together as there's no adapter method defined on Stream directly
        // for ForEachConcurrentClientUse
        let used_clients = ForEachConcurrentClientUse::new(
            stream::iter(stream_data.into_iter()),
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
        .flatten()
        .collect();

        self.merge_client_handles(used_clients).await;
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
    result: Vec<Option<GatewayClientHandle>>,
}

impl<St, Fut, F> ForEachConcurrentClientUse<St, Fut, F>
where
    St: Stream,
    F: FnMut(St::Item) -> Fut,
    Fut: Future<Output = Option<GatewayClientHandle>>,
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
    Fut: Future<Output = Option<GatewayClientHandle>>,
{
    type Output = Vec<Option<GatewayClientHandle>>;

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
