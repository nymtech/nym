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

use crate::client::inbound_messages::InputMessageReceiver;
use crate::client::real_messages_control::acknowlegement_control::acknowledgement_listener::AcknowledgementListener;
use crate::client::real_messages_control::acknowlegement_control::input_message_listener::InputMessageListener;
use crate::client::real_messages_control::acknowlegement_control::retransmission_request_listener::RetransmissionRequestListener;
use crate::client::real_messages_control::acknowlegement_control::sent_notification_listener::SentNotificationListener;
use crate::client::real_traffic_stream::RealSphinxSender;
use crate::client::topology_control::{TopologyAccessor, TopologyReadPermit};
use futures::channel::mpsc;
use log::*;
use nymsphinx::acknowledgements;
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

mod acknowledgement_listener;
mod input_message_listener;
mod retransmission_request_listener;
mod sent_notification_listener;

pub(crate) type AcknowledgementSender = mpsc::UnboundedSender<Vec<u8>>;
pub(crate) type AcknowledgementReceiver = mpsc::UnboundedReceiver<Vec<u8>>;

type RetransmissionRequestSender = mpsc::UnboundedSender<FragmentIdentifier>;
type RetransmissionRequestReceiver = mpsc::UnboundedReceiver<FragmentIdentifier>;

pub(crate) type SentPacketNotificationSender = mpsc::UnboundedSender<FragmentIdentifier>;
type SentPacketNotificationReceiver = mpsc::UnboundedReceiver<FragmentIdentifier>;

type PendingAcksMap = Arc<RwLock<HashMap<FragmentIdentifier, PendingAcknowledgement>>>;

struct PendingAcknowledgement {
    message_chunk: Fragment,
    delay: Delay,
    recipient: Recipient,
    retransmission_cancel: Arc<Notify>,
}

impl PendingAcknowledgement {
    fn new(message_chunk: Fragment, delay: Delay, recipient: Recipient) -> Self {
        PendingAcknowledgement {
            message_chunk,
            delay,
            retransmission_cancel: Arc::new(Notify::new()),
            recipient,
        }
    }

    fn update_delay(&mut self, new_delay: Delay) {
        self.delay = new_delay;
    }
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

pub(crate) struct AcknowledgementControllerConnectors {
    real_sphinx_sender: RealSphinxSender,
    input_receiver: InputMessageReceiver,
    sent_notifier: SentPacketNotificationReceiver,
    ack_receiver: AcknowledgementReceiver,
}

pub(crate) struct AcknowledgementController<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    acknowledgement_listener: Option<AcknowledgementListener>,
    input_message_listener: Option<InputMessageListener<R, T>>,
    retransmission_request_listener: Option<RetransmissionRequestListener<R, T>>,
    sent_notification_listener: Option<SentNotificationListener>,
}

impl<T: 'static + NymTopology> AcknowledgementController<OsRng, T> {
    pub(crate) fn new(
        topology_access: TopologyAccessor<T>,
        ack_recipient: Recipient,
        average_packet_delay_duration: Duration,
        average_ack_delay_duration: Duration,
        connectors: AcknowledgementControllerConnectors,
    ) -> Self {
        let mut rng = OsRng;

        // note for future-self: perhaps for key rotation we could replace it with Arc<AtomicCell<Key>> ?
        // actually same could be true for any keys we use
        let ack_key = Arc::new(acknowledgements::generate_key(&mut rng));
        let pending_acks = Arc::new(RwLock::new(HashMap::new()));
        let message_chunker = MessageChunker::new_with_rng(
            rng,
            ack_recipient.clone(),
            average_packet_delay_duration,
            average_ack_delay_duration,
        );

        let acknowledgement_listener = AcknowledgementListener::new(
            Arc::clone(&ack_key),
            connectors.ack_receiver,
            Arc::clone(&pending_acks),
        );

        let input_message_listener = InputMessageListener::new(
            Arc::clone(&ack_key),
            ack_recipient.clone(),
            connectors.input_receiver,
            message_chunker.clone(),
            Arc::clone(&pending_acks),
            connectors.real_sphinx_sender.clone(),
            topology_access.clone(),
        );

        let (retransmission_tx, retransmission_rx) = mpsc::unbounded();

        let retransmission_request_listener = RetransmissionRequestListener::new(
            ack_key,
            ack_recipient,
            message_chunker,
            Arc::clone(&pending_acks),
            connectors.real_sphinx_sender,
            retransmission_rx,
            topology_access,
        );

        let sent_notification_listener = SentNotificationListener::new(
            connectors.sent_notifier,
            pending_acks,
            retransmission_tx,
        );

        AcknowledgementController {
            acknowledgement_listener: Some(acknowledgement_listener),
            input_message_listener: Some(input_message_listener),
            retransmission_request_listener: Some(retransmission_request_listener),
            sent_notification_listener: Some(sent_notification_listener),
        }
    }

    pub(crate) async fn run(&mut self) {
        let mut acknowledgement_listener = self.acknowledgement_listener.take().unwrap();
        let mut input_message_listener = self.input_message_listener.take().unwrap();
        let mut retransmission_request_listener =
            self.retransmission_request_listener.take().unwrap();
        let mut sent_notification_listener = self.sent_notification_listener.take().unwrap();

        // TODO: perhaps an extra 'DEBUG' task that would periodically check for stale entries in
        // pending acks map?
        // It would only be 'DEBUG' as I don't expect any stale entries to exist there to begin with,
        // but when can bugs be expected to begin with?

        // the below are log messages are errors as at the current stage we do not expect any of
        // the task to ever finish. This will of course change once we introduce
        // graceful shutdowns.
        let ack_listener_fut = tokio::spawn(async move {
            acknowledgement_listener.run().await;
            error!("The acknowledgement listener has finished execution!");
            acknowledgement_listener
        });
        let input_listener_fut = tokio::spawn(async move {
            input_message_listener.run().await;
            error!("The input listener has finished execution!");
            input_message_listener
        });
        let retransmission_req_fut = tokio::spawn(async move {
            retransmission_request_listener.run().await;
            error!("The retransmission request listener has finished execution!");
            retransmission_request_listener
        });
        let sent_notification_fut = tokio::spawn(async move {
            sent_notification_listener.run().await;
            error!("The sent notification listener has finished execution!");
            sent_notification_listener
        });

        // technically we don't have to bring `AcknowledgementController` back to a valid state
        // but we can do it, so why not? Perhaps it might be useful if we wanted to allow
        // for restarts of certain modules without killing the entire process.
        self.acknowledgement_listener = Some(ack_listener_fut.await.unwrap());
        self.input_message_listener = Some(input_listener_fut.await.unwrap());
        self.retransmission_request_listener = Some(retransmission_req_fut.await.unwrap());
        self.sent_notification_listener = Some(sent_notification_fut.await.unwrap());
    }

    // &Handle is only passed for consistency sake with other client modules, but I think
    // when we get to refactoring, we should apply gateway approach and make it implicit
    pub(crate) fn start(mut self, handle: &Handle) -> JoinHandle<Self> {
        handle.spawn(async move {
            self.run().await;
            self
        })
    }
}
