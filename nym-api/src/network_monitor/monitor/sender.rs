// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::monitor::gateway_client_handle::GatewayClientHandle;
use crate::network_monitor::monitor::receiver::{GatewayClientUpdate, GatewayClientUpdateSender};
use crate::support::config::Config;
use crate::support::nyxd;
use dashmap::DashMap;
use futures::channel::mpsc;
use futures::stream::{self, FuturesUnordered, StreamExt};
use futures::task::Context;
use futures::{Future, Stream};
use nym_bandwidth_controller::BandwidthController;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_client::client::config::GatewayClientConfig;
use nym_gateway_client::client::GatewayConfig;
use nym_gateway_client::error::GatewayClientError;
use nym_gateway_client::{
    AcknowledgementReceiver, GatewayClient, MixnetMessageReceiver, PacketRouter, SharedGatewayKey,
};
use nym_sphinx::forwarding::packet::MixPacket;
use pin_project::pin_project;
use sqlx::__rt::timeout;
use std::mem;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tracing::{debug, error, info, trace, warn};

const TIME_CHUNK_SIZE: Duration = Duration::from_millis(50);

pub(crate) struct GatewayPackets {
    /// Network address of the target gateway if wanted to be accessed by the client.
    /// It is a websocket address.
    pub(crate) clients_address: String,

    /// Public key of the target gateway.
    pub(crate) pub_key: ed25519::PublicKey,

    /// All the packets that are going to get sent to the gateway.
    pub(crate) packets: Vec<MixPacket>,
}

impl GatewayPackets {
    pub(crate) fn new(
        clients_address: String,
        pub_key: ed25519::PublicKey,
        packets: Vec<MixPacket>,
    ) -> Self {
        GatewayPackets {
            clients_address,
            pub_key,
            packets,
        }
    }

    pub(crate) fn gateway_config(&self) -> GatewayConfig {
        GatewayConfig {
            gateway_identity: self.pub_key,
            gateway_owner: None,
            gateway_listener: self.clients_address.clone(),
        }
    }

    pub(crate) fn empty(clients_address: String, pub_key: ed25519::PublicKey) -> Self {
        GatewayPackets {
            clients_address,
            pub_key,
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
    local_identity: Arc<ed25519::KeyPair>,
    gateway_response_timeout: Duration,
    bandwidth_controller: BandwidthController<nyxd::Client, PersistentStorage>,
    disabled_credentials_mode: bool,
    gateways_key_cache: DashMap<ed25519::PublicKey, Arc<SharedGatewayKey>>,
}

impl FreshGatewayClientData {
    fn notify_new_connection(
        self: Arc<FreshGatewayClientData>,
        gateway_id: ed25519::PublicKey,
        gateway_channels: (MixnetMessageReceiver, AcknowledgementReceiver),
    ) {
        if self
            .gateways_status_updater
            .unbounded_send(GatewayClientUpdate::New(gateway_id, gateway_channels))
            .is_err()
        {
            error!("packet receiver seems to have died!")
        }
    }
}

pub(crate) struct PacketSender {
    fresh_gateway_client_data: Arc<FreshGatewayClientData>,
    gateway_connection_timeout: Duration,
    gateway_bandwidth_claim_timeout: Duration,
    max_concurrent_clients: usize,
    max_sending_rate: usize,
}

impl PacketSender {
    pub(crate) fn new(
        config: &Config,
        gateways_status_updater: GatewayClientUpdateSender,
        local_identity: Arc<ed25519::KeyPair>,
        bandwidth_controller: BandwidthController<nyxd::Client, PersistentStorage>,
    ) -> Self {
        PacketSender {
            fresh_gateway_client_data: Arc::new(FreshGatewayClientData {
                gateways_status_updater,
                local_identity,
                gateway_response_timeout: config.network_monitor.debug.gateway_response_timeout,
                bandwidth_controller,
                disabled_credentials_mode: config.network_monitor.debug.disabled_credentials_mode,
                gateways_key_cache: Default::default(),
            }),
            gateway_connection_timeout: config.network_monitor.debug.gateway_connection_timeout,
            gateway_bandwidth_claim_timeout: config
                .network_monitor
                .debug
                .gateway_bandwidth_claim_timeout,
            max_concurrent_clients: config.network_monitor.debug.max_concurrent_gateway_clients,
            max_sending_rate: config.network_monitor.debug.gateway_sending_rate,
        }
    }

    fn new_gateway_client_handle(
        config: GatewayConfig,
        fresh_gateway_client_data: &FreshGatewayClientData,
    ) -> (
        GatewayClientHandle,
        (MixnetMessageReceiver, AcknowledgementReceiver),
    ) {
        // I think the proper one should be passed around instead...
        let task_client =
            nym_task::TaskClient::dummy().named(format!("gateway-{}", config.gateway_identity));

        let (message_sender, message_receiver) = mpsc::unbounded();

        // currently we do not care about acks at all, but we must keep the channel alive
        // so that the gateway client would not crash
        let (ack_sender, ack_receiver) = mpsc::unbounded();

        let gateway_packet_router = PacketRouter::new(
            ack_sender,
            message_sender,
            task_client.fork("packet-router"),
        );

        let shared_keys = fresh_gateway_client_data
            .gateways_key_cache
            .get(&config.gateway_identity)
            .map(|k| k.value().clone());

        let gateway_client = GatewayClient::new(
            GatewayClientConfig::new_default()
                .with_disabled_credentials_mode(fresh_gateway_client_data.disabled_credentials_mode)
                .with_response_timeout(fresh_gateway_client_data.gateway_response_timeout),
            config,
            Arc::clone(&fresh_gateway_client_data.local_identity),
            shared_keys,
            gateway_packet_router,
            Some(fresh_gateway_client_data.bandwidth_controller.clone()),
            nym_statistics_common::clients::ClientStatsSender::new(None),
            task_client,
        );

        (
            GatewayClientHandle::new(
                gateway_client,
                fresh_gateway_client_data.gateways_status_updater.clone(),
            ),
            (message_receiver, ack_receiver),
        )
    }

    async fn attempt_to_send_packets(
        client: &mut GatewayClient<nyxd::Client, PersistentStorage>,
        mut mix_packets: Vec<MixPacket>,
        max_sending_rate: usize,
    ) -> Result<(), GatewayClientError> {
        let gateway_id = client.gateway_identity();

        info!(
            "Got {} packets to send to gateway {gateway_id}",
            mix_packets.len(),
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

    async fn client_startup(
        connection_timeout: Duration,
        bandwidth_claim_timeout: Duration,
        client: &mut GatewayClientHandle,
    ) -> Option<Arc<SharedGatewayKey>> {
        let gateway_identity = client.gateway_identity();

        // 1. attempt to authenticate
        let shared_key =
            match timeout(connection_timeout, client.perform_initial_authentication()).await {
                Err(_timeout) => {
                    warn!("timed out while trying to authenticate with gateway {gateway_identity}");
                    return None;
                }
                Ok(Err(err)) => {
                    warn!("failed to authenticate with gateway ({gateway_identity}): {err}");
                    return None;
                }
                Ok(Ok(res)) => res.initial_shared_key,
            };

        // 2. maybe claim bandwidth
        match timeout(bandwidth_claim_timeout, client.claim_initial_bandwidth()).await {
            Err(_timeout) => {
                warn!("timed out while trying to claim initial bandwidth with gateway {gateway_identity}");
                return None;
            }
            Ok(Err(err)) => {
                warn!("failed to claim bandwidth with gateway ({gateway_identity}): {err}");
                return None;
            }
            Ok(Ok(_)) => (),
        }

        // 3. start internal listener
        if let Err(err) = client.start_listening_for_mixnet_messages() {
            warn!("failed to start message listener for {gateway_identity}: {err}");
            return None;
        }

        Some(shared_key)
    }

    async fn create_new_gateway_client_handle_and_authenticate(
        config: GatewayConfig,
        fresh_gateway_client_data: &FreshGatewayClientData,
        gateway_connection_timeout: Duration,
        gateway_bandwidth_claim_timeout: Duration,
    ) -> Option<(
        GatewayClientHandle,
        (MixnetMessageReceiver, AcknowledgementReceiver),
    )> {
        let gateway_identity = config.gateway_identity;
        let (mut new_client, (message_receiver, ack_receiver)) =
            Self::new_gateway_client_handle(config, fresh_gateway_client_data);

        match Self::client_startup(
            gateway_connection_timeout,
            gateway_bandwidth_claim_timeout,
            &mut new_client,
        )
        .await
        {
            Some(shared_key) => {
                fresh_gateway_client_data
                    .gateways_key_cache
                    .insert(gateway_identity, shared_key);
                Some((new_client, (message_receiver, ack_receiver)))
            }
            None => {
                fresh_gateway_client_data
                    .gateways_key_cache
                    .remove(&gateway_identity);
                None
            }
        }
    }

    async fn check_remaining_bandwidth(
        client: &mut GatewayClient<nyxd::Client, PersistentStorage>,
    ) -> Result<(), GatewayClientError> {
        if client.remaining_bandwidth() < client.cfg.bandwidth.remaining_bandwidth_threshold {
            Err(GatewayClientError::NotEnoughBandwidth(
                client.cfg.bandwidth.remaining_bandwidth_threshold,
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
        gateway_bandwidth_claim_timeout: Duration,
        packets: GatewayPackets,
        fresh_gateway_client_data: Arc<FreshGatewayClientData>,
        max_sending_rate: usize,
    ) -> Option<GatewayClientHandle> {
        let (mut client, gateway_channels) =
            Self::create_new_gateway_client_handle_and_authenticate(
                packets.gateway_config(),
                &fresh_gateway_client_data,
                gateway_connection_timeout,
                gateway_bandwidth_claim_timeout,
            )
            .await?;

        let identity = client.gateway_identity();

        let estimated_time =
            Duration::from_secs_f64(packets.packets.len() as f64 / max_sending_rate as f64);
        // give some leeway
        let timeout = estimated_time * 3;

        if let Err(err) = Self::check_remaining_bandwidth(&mut client).await {
            warn!("Failed to claim additional bandwidth for {identity}: {err}",);
            return None;
        }

        match tokio::time::timeout(
            timeout,
            Self::attempt_to_send_packets(&mut client, packets.packets, max_sending_rate),
        )
        .await
        {
            Err(_timeout) => {
                warn!("failed to send packets to {identity} - we timed out",);
                return None;
            }
            Ok(Err(err)) => {
                warn!("failed to send packets to {identity}: {err}",);
                return None;
            }
            Ok(Ok(_)) => {
                fresh_gateway_client_data.notify_new_connection(identity, gateway_channels)
            }
        }

        Some(client)
    }

    pub(super) async fn send_packets(
        &mut self,
        packets: Vec<GatewayPackets>,
    ) -> Vec<GatewayClientHandle> {
        // we know that each of the elements in the packets array will only ever access a single,
        // unique element from the existing clients

        let gateway_connection_timeout = self.gateway_connection_timeout;
        let gateway_bandwidth_claim_timeout = self.gateway_bandwidth_claim_timeout;
        let max_concurrent_clients = if self.max_concurrent_clients > 0 {
            Some(self.max_concurrent_clients)
        } else {
            None
        };
        let max_sending_rate = self.max_sending_rate;

        let stream_data = packets
            .into_iter()
            .map(|packets| (packets, Arc::clone(&self.fresh_gateway_client_data)))
            .collect::<Vec<_>>();

        // can't chain it all nicely together as there's no adapter method defined on Stream directly
        // for ForEachConcurrentClientUse
        //
        // we need to keep clients alive until the test finishes so that we could keep receiving
        ForEachConcurrentClientUse::new(
            stream::iter(stream_data.into_iter()),
            max_concurrent_clients,
            |(packets, fresh_data)| async move {
                Self::send_gateway_packets(
                    gateway_connection_timeout,
                    gateway_bandwidth_claim_timeout,
                    packets,
                    fresh_data,
                    max_sending_rate,
                )
                .await
            },
        )
        .await
        .into_iter()
        .flatten()
        .collect()
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
