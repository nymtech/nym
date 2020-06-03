// Copyright 2020 Nym Technologies SA
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

use crate::client::inbound_messages::{InputMessage, InputMessageReceiver};
use crate::client::real_traffic_stream::RealSphinxSender;
use crate::client::topology_control::{TopologyAccessor, TopologyReadPermit};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nymsphinx::acknowledgements::{self, identifier::recover_identifier, AckAes128Key};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::chunking::{
    fragment::{Fragment, FragmentIdentifier},
    MessageChunker,
};
use nymsphinx::Delay;
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use topology::NymTopology;

pub(crate) type AcknowledgementSender = mpsc::UnboundedSender<Vec<u8>>;
pub(crate) type AcknowledgementReceiver = mpsc::UnboundedReceiver<Vec<u8>>;

type RetransmissionRequestSender = mpsc::UnboundedSender<FragmentIdentifier>;
type RetransmissionRequestReceiver = mpsc::UnboundedReceiver<FragmentIdentifier>;

pub(crate) type SentPacketNotificationSender = mpsc::UnboundedSender<FragmentIdentifier>;
type SentPacketNotificationReceiver = mpsc::UnboundedReceiver<FragmentIdentifier>;

struct PendingAcknowledgement {
    fragment: Fragment,
    delay: Delay,
    recipient: Recipient,
    retransmission_cancel: Arc<Notify>,
}

impl PendingAcknowledgement {
    fn update_delay(&mut self, new_delay: Delay) {
        self.delay = new_delay;
    }
}

type PendingAcksMap = Arc<RwLock<HashMap<FragmentIdentifier, PendingAcknowledgement>>>;

pub(crate) struct AcknowledgementController<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    // note for future-self: perhaps for key rotation we could replace it with Arc<AtomicCell<Key>> ?
    // actually same could be true for any keys we use
    ack_key: Arc<AckAes128Key>,
    ack_recipient: Recipient,
    pending_acks: PendingAcksMap,
    message_chunker: MessageChunker<R>,
    topology_access: TopologyAccessor<T>,

    // TODOs:
    real_sphinx_sender: RealSphinxSender,
}

// Using provided topology read permit, tries to get an immutable reference to the underlying
// topology. For obvious reasons the lifetime of the topology reference is bound to the permit.
fn try_get_valid_topology_ref<'a, T: NymTopology>(
    ack_recipient: &Recipient,
    packet_recipient: &Recipient,
    topology_permit: &'a TopologyReadPermit<'_, T>,
) -> Option<&'a T> {
    // first we need to deref out of RwLockReadGuard
    // then we need to deref out of TopologyAccessorInner
    // then we must take ref of option, i.e. Option<&T>
    // and finally try to unwrap it to obtain &T
    let topology_ref_option = (*topology_permit.deref()).as_ref();

    if topology_ref_option.is_none() {
        return None;
    }

    let topology_ref = topology_ref_option.unwrap();

    // see if it's possible to route the packet to both gateways
    if !topology_ref.can_construct_path_through()
        || !topology_ref.gateway_exists(&packet_recipient.gateway())
        || !topology_ref.gateway_exists(&ack_recipient.gateway())
    {
        None
    } else {
        Some(topology_ref)
    }
}

impl<T: 'static + NymTopology> AcknowledgementController<OsRng, T> {
    pub(crate) fn new(
        topology_access: TopologyAccessor<T>,
        ack_recipient: Recipient,
        average_packet_delay_duration: Duration,
        average_ack_delay_duration: Duration,
    ) -> Self {
        todo!()

        // let mut rng = OsRng;
        // AcknowledgementController {
        //     ack_key: Arc::new(acknowledgements::generate_key(&mut rng)),
        //     ack_recipient: ack_recipient.clone(),
        //     pending_acks: Arc::new(Mutex::new(HashMap::new())),
        //     message_chunker: MessageChunker::new_with_rng(
        //         rng,
        //         ack_recipient,
        //         average_packet_delay_duration,
        //         average_ack_delay_duration,
        //     ),
        //     topology_access,
        // }
    }

    pub(crate) async fn run(&mut self) {
        // TODO: perhaps an extra 'DEBUG' task that would periodically check for stale entries in
        // pending acks map?
        // It would only be 'DEBUG' as I don't expect any stale entries to exist there, but
        // when can bugs be expected to begin with?

        // start all modules here

        // tokio::join!(...)

        todo!()
    }

    // &Handle is only passed for consistency sake with other client modules, but I think
    // when we get to refactoring, we should apply gateway approach and make it implicit
    pub(crate) fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move { self.run().await })
    }
}

// responsible for splitting received message and initial sending attempt
struct TMP_NAME_InputMessageReceiver<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    pending_acks: PendingAcksMap,
    ack_recipient: Recipient,
    topology_access: TopologyAccessor<T>,
    ack_key: Arc<AckAes128Key>,
    real_sphinx_sender: RealSphinxSender,
    message_chunker: MessageChunker<R>,
}

impl<R, T> TMP_NAME_InputMessageReceiver<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    async fn on_input_message(&mut self, msg: InputMessage) {
        let (recipient, content) = msg.destruct();
        let split_message = self.message_chunker.split_message(&content);
        let topology_permit = &self.topology_access.get_read_permit().await;

        let topology_ref_option =
            try_get_valid_topology_ref(&self.ack_recipient, &recipient, topology_permit);
        if topology_ref_option.is_none() {
            warn!("Could not process the message - the network topology is invalid");
            return;
        }
        let topology_ref = topology_ref_option.unwrap();

        let mut pending_acks = Vec::with_capacity(split_message.len());

        for message_chunk in split_message {
            // since the paths can be constructed, this CAN'T fail, if it does, there's a bug somewhere
            let frag_id = message_chunk.fragment_identifier();
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = message_chunk.clone();
            let (total_delay, packet) = self
                .message_chunker
                .prepare_chunk_for_sending(chunk_clone, topology_ref, &self.ack_key, &recipient)
                .unwrap();

            self.real_sphinx_sender.unbounded_send(packet).unwrap();

            let pending_ack = PendingAcknowledgement {
                fragment: message_chunk,
                delay: total_delay,
                retransmission_cancel: Arc::new(Notify::new()),
                recipient: recipient.clone(),
            };
            pending_acks.push((frag_id, pending_ack));
        }

        let mut pending_acks_map_write_guard = self.pending_acks.write().await;
        for (frag_id, pending_ack) in pending_acks.into_iter() {
            if let Some(_) = pending_acks_map_write_guard.insert(frag_id, pending_ack) {
                panic!("Tried to insert duplicate pending ack")
            }
        }
    }

    async fn run(&mut self, mut input_receiver: InputMessageReceiver) {
        while let Some(input_msg) = input_receiver.next().await {
            self.on_input_message(input_msg);
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}

// responsible for cancelling retransmission timers and removed entries from the map
struct TMP_NAME_AcknowledgementReceiver {
    ack_key: Arc<AckAes128Key>,
    pending_acks: PendingAcksMap,
}

impl TMP_NAME_AcknowledgementReceiver {
    async fn on_ack(&mut self, ack_content: Vec<u8>) {
        let frag_id = match recover_identifier(&self.ack_key, &ack_content) {
            None => {
                warn!("Received invalid ACK!"); // should we do anything else about that?
                return;
            }
            Some(frag_id_bytes) => match FragmentIdentifier::try_from_bytes(&frag_id_bytes) {
                Ok(frag_id) => frag_id,
                Err(err) => {
                    warn!("Received invalid ACK! - {:?}", err); // should we do anything else about that?
                    return;
                }
            },
        };

        // TODO: check if ack for cover message once cover messages include acks
        // I guess they will probably have (0i32,0u8) because both of those values are invalid
        // for normal fragments?

        if let Some(pending_ack) = self.pending_acks.write().await.remove(&frag_id) {
            // cancel the retransmission future
            pending_ack.retransmission_cancel.notify();
        } else {
            warn!("received ACK for packet we haven't stored! - {:?}", frag_id);
        }
    }

    async fn listen_for_acknowledgements(&mut self, mut ack_receiver: AcknowledgementReceiver) {
        while let Some(ack) = ack_receiver.next().await {
            self.on_ack(ack);
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}

// responsible for packet retransmission upon fired timer
struct TMP_NAME_RetransmissionRequestReceiver<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    pending_acks: PendingAcksMap,
    ack_recipient: Recipient,
    topology_access: TopologyAccessor<T>,
    ack_key: Arc<AckAes128Key>,
    real_sphinx_sender: RealSphinxSender,
    message_chunker: MessageChunker<R>,
}

impl<R, T> TMP_NAME_RetransmissionRequestReceiver<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    async fn on_retransmission_request(&mut self, frag_id: FragmentIdentifier) {
        let pending_acks_map_read_guard = self.pending_acks.read().await;
        // if the unwrap failed here, we have some weird bug somewhere - honestly, I'm not sure
        // if it's even possible for it to happen
        let unreceived_ack_fragment = pending_acks_map_read_guard
            .get(&frag_id)
            .expect("wanted to retransmit ack'd fragment");

        let packet_recipient = unreceived_ack_fragment.recipient.clone();
        let chunk_clone = unreceived_ack_fragment.fragment.clone();

        // TODO: we need some proper benchmarking here to determine whether it could
        // be more efficient to just get write lock and keep it while doing sphinx computation,
        // but my gut feeling tells me we should re-acquire it.
        drop(pending_acks_map_read_guard);

        let topology_permit = &self.topology_access.get_read_permit().await;
        let topology_ref_option =
            try_get_valid_topology_ref(&self.ack_recipient, &packet_recipient, topology_permit);
        if topology_ref_option.is_none() {
            warn!("Could not retransmit the packet - the network topology is invalid");
            // TODO: perhaps put back into pending acks and reset the timer?
            return;
        }
        let topology_ref = topology_ref_option.unwrap();

        let (total_delay, packet) = self
            .message_chunker
            .prepare_chunk_for_sending(chunk_clone, topology_ref, &self.ack_key, &packet_recipient)
            .unwrap();

        self.real_sphinx_sender.unbounded_send(packet).unwrap();

        self.pending_acks
            .write()
            .await
            .get_mut(&frag_id)
            .expect(
                "on_retransmission_request: somehow we already received an ack for this packet?",
            )
            .update_delay(total_delay);
    }

    async fn run(&mut self, mut req_receiver: RetransmissionRequestReceiver) {
        while let Some(frag_id) = req_receiver.next().await {
            self.on_retransmission_request(frag_id);
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}

// responsible for starting and controlling retransmission timers
// it is required because when we send our packet to the `real traffic stream` controlled
// with poisson timer, there's no guarantee the message will be sent immediately, so we might
// accidentally fire retransmission way quicker than we would have wanted.
struct TMP_NAME_SentNotificationReceiver {
    pending_acks: PendingAcksMap,
    retransmission_sender: RetransmissionRequestSender,
}

impl TMP_NAME_SentNotificationReceiver {
    async fn on_sent_message(&mut self, frag_id: FragmentIdentifier) {
        let pending_acks_map_read_guard = self.pending_acks.read().await;
        // if the unwrap failed here, we have some weird bug somewhere
        // although when I think about it, it *theoretically* could happen under extremely heavy client
        // load that `on_sent_message()` is not called (and we do not receive the read permit)
        // until we already received and processed an ack for the packet
        // but this seems extremely unrealistic, but perhaps we should guard against that?
        let pending_ack_data = pending_acks_map_read_guard
            .get(&frag_id)
            .expect("on_sent_message: somehow we already received an ack for this packet?");

        // if this assertion ever fails, we have some bug due to some unintended leak.
        // the only reason I see it could happen if the `tokio::select` in the spawned
        // task below somehow did not drop it
        debug_assert_eq!(
            Arc::strong_count(&pending_ack_data.retransmission_cancel),
            1
        );

        // TODO: read more about Arc::downgrade. it could be useful here
        let retransmission_cancel = Arc::clone(&pending_ack_data.retransmission_cancel);

        // TODO: put the retransmission_timeout constants in config file
        let retransmission_timeout =
            tokio::time::delay_for(pending_ack_data.delay.to_duration() + Duration::from_secs(2));

        let retransmission_sender = self.retransmission_sender.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = retransmission_cancel.notified() => {
                    trace!("received ack for the fragment. Cancelling retransmission future");
                }
                _ = retransmission_timeout => {
                    trace!("did not receive an ack - will retransmit the packet");
                    retransmission_sender.unbounded_send(frag_id).unwrap();
                }
            }
        });
    }

    async fn run(&mut self, mut sent_notifier: SentPacketNotificationReceiver) {
        while let Some(frag_id) = sent_notifier.next().await {
            self.on_sent_message(frag_id).await;
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}

// required module IO:
// 1. receive from input
// 2. send to real traffic stream
// 3. receive oneshot or notify? from RTS once sent; alternatively maybe mpsc<id> ?
