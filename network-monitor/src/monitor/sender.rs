// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::monitor::receiver::{GatewayClientUpdate, GatewayClientUpdateSender};
use crypto::asymmetric::identity::{self, PUBLIC_KEY_LENGTH};
use futures::channel::mpsc;
use futures::stream::{self, FuturesUnordered, StreamExt};
use futures::task::Context;
use futures::{Future, Stream};
use gateway_client::{GatewayClient, MixnetMessageReceiver};
use log::*;
use nymsphinx::forwarding::packet::MixPacket;
use pin_project::pin_project;
use std::collections::HashMap;
use std::mem;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

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

    pub(super) fn gateway_address(&self) -> identity::PublicKey {
        self.pub_key
    }
}

// struct consisting of all external data required to construct a fresh gateway client
struct FreshGatewayClientData {
    gateways_status_updater: GatewayClientUpdateSender,
    local_identity: Arc<identity::KeyPair>,
    gateway_response_timeout: Duration,
}

pub(crate) struct PacketSender {
    // TODO: this has a potential long-term issue. If we keep those clients cached between runs,
    // malicious gateways could figure out which traffic comes from the network monitor and always
    // forward that traffic while dropping the rest. However, at the current stage such sophisticated
    // behaviour is unlikely.
    active_gateway_clients: HashMap<[u8; PUBLIC_KEY_LENGTH], GatewayClient>,

    fresh_gateway_client_data: Arc<FreshGatewayClientData>,
    max_concurrent_clients: Option<usize>,
}

impl PacketSender {
    pub(crate) fn new(
        gateways_status_updater: GatewayClientUpdateSender,
        local_identity: Arc<identity::KeyPair>,
        gateway_response_timeout: Duration,
        max_concurrent_clients: Option<usize>,
    ) -> Self {
        PacketSender {
            active_gateway_clients: HashMap::new(),
            fresh_gateway_client_data: Arc::new(FreshGatewayClientData {
                gateways_status_updater,
                local_identity,
                gateway_response_timeout,
            }),
            max_concurrent_clients,
        }
    }

    fn new_gateway_client(
        address: String,
        identity: identity::PublicKey,
        fresh_gateway_client_data: &FreshGatewayClientData,
    ) -> (GatewayClient, MixnetMessageReceiver) {
        // TODO: future optimization: if we're remaking client for a gateway to which we used to be connected in the past,
        // use old shared keys
        let (message_sender, message_receiver) = mpsc::unbounded();
        // currently we do not care about acks at all
        let (ack_sender, _) = mpsc::unbounded();
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
            message_receiver,
        )
    }

    // TODO: perhaps it should be spawned as a task to execute it in parallel rather
    // than just concurrently?
    async fn send_gateway_packets(
        packets: GatewayPackets,
        fresh_gateway_client_data: Arc<FreshGatewayClientData>,
        client: Option<GatewayClient>,
    ) -> Option<GatewayClient> {
        let was_present = client.is_some();

        let (mut client, message_receiver) = if let Some(client) = client {
            (client, None)
        } else {
            let (new_client, message_receiver) = Self::new_gateway_client(
                packets.clients_address,
                packets.pub_key,
                &fresh_gateway_client_data,
            );
            (new_client, Some(message_receiver))
        };

        // TODO: change and introduce rate limiting like in the old code
        if let Err(err) = client.batch_send_mix_packets(packets.packets).await {
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
                    message_receiver.expect("we created a new client, yet the channel is a None!"),
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
        let max_concurrent_clients = self.max_concurrent_clients;

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
                Self::send_gateway_packets(packets, fresh_data, client).await
            },
        )
        .await
        .into_iter()
        .for_each(|client| {
            if let Some(client) = client {
                if let Some(existing) = self
                    .active_gateway_clients
                    .insert(client.identity().to_bytes(), client)
                {
                    // TODO: perhaps panic instead? getting here implies there's some serious logic
                    // error somewhere and our assumptions no longer hold
                    error!(
                        "we got duplicate gateway client for {}!",
                        existing.identity().to_base58_string()
                    );
                }
            }
        })
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
