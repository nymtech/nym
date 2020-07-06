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

use self::{
    acknowledgement_listener::AcknowledgementListener,
    input_message_listener::InputMessageListener,
    retransmission_request_listener::RetransmissionRequestListener,
    sent_notification_listener::SentNotificationListener,
};
use super::real_traffic_stream::RealMessageSender;
use crate::client::{inbound_messages::InputMessageReceiver, topology_control::TopologyAccessor};
use futures::channel::mpsc;
use gateway_client::AcknowledgementReceiver;
use log::*;
use nymsphinx::{
    acknowledgements::{self, identifier::AckAes128Key},
    addressing::clients::Recipient,
    chunking::{
        fragment::{Fragment, FragmentIdentifier},
        MessageChunker,
    },
    Delay,
};
use rand::{CryptoRng, Rng};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::{Notify, RwLock},
    task::JoinHandle,
};

mod acknowledgement_listener;
mod input_message_listener;
mod retransmission_request_listener;
mod sent_notification_listener;

type RetransmissionRequestSender = mpsc::UnboundedSender<FragmentIdentifier>;
type RetransmissionRequestReceiver = mpsc::UnboundedReceiver<FragmentIdentifier>;

pub(super) type SentPacketNotificationSender = mpsc::UnboundedSender<FragmentIdentifier>;
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

pub(super) struct AcknowledgementControllerConnectors {
    real_message_sender: RealMessageSender,
    input_receiver: InputMessageReceiver,
    sent_notifier: SentPacketNotificationReceiver,
    ack_receiver: AcknowledgementReceiver,
}

impl AcknowledgementControllerConnectors {
    pub(super) fn new(
        real_message_sender: RealMessageSender,
        input_receiver: InputMessageReceiver,
        sent_notifier: SentPacketNotificationReceiver,
        ack_receiver: AcknowledgementReceiver,
    ) -> Self {
        AcknowledgementControllerConnectors {
            real_message_sender,
            input_receiver,
            sent_notifier,
            ack_receiver,
        }
    }
}

pub(super) struct AcknowledgementController<R>
where
    R: CryptoRng + Rng,
{
    ack_key: Arc<AckAes128Key>,
    acknowledgement_listener: Option<AcknowledgementListener>,
    input_message_listener: Option<InputMessageListener<R>>,
    retransmission_request_listener: Option<RetransmissionRequestListener<R>>,
    sent_notification_listener: Option<SentNotificationListener>,
}

impl<R> AcknowledgementController<R>
where
    R: 'static + CryptoRng + Rng + Clone + Send,
{
    pub(super) fn new(
        mut rng: R,
        topology_access: TopologyAccessor,
        ack_recipient: Recipient,
        average_packet_delay_duration: Duration,
        average_ack_delay_duration: Duration,
        ack_wait_multiplier: f64,
        ack_wait_addition: Duration,
        connectors: AcknowledgementControllerConnectors,
    ) -> Self {
        // note for future-self: perhaps for key rotation we could replace it with Arc<AtomicCell<Key>> ?
        // actually same could be true for any keys we use
        let ack_key = Arc::new(acknowledgements::generate_key(&mut rng));
        let pending_acks = Arc::new(RwLock::new(HashMap::new()));
        let message_chunker = MessageChunker::new_with_rng(
            rng,
            ack_recipient.clone(),
            true,
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
            connectors.real_message_sender.clone(),
            topology_access.clone(),
        );

        let (retransmission_tx, retransmission_rx) = mpsc::unbounded();

        let retransmission_request_listener = RetransmissionRequestListener::new(
            Arc::clone(&ack_key),
            ack_recipient,
            message_chunker,
            Arc::clone(&pending_acks),
            connectors.real_message_sender,
            retransmission_rx,
            topology_access,
        );

        let sent_notification_listener = SentNotificationListener::new(
            ack_wait_multiplier,
            ack_wait_addition,
            connectors.sent_notifier,
            pending_acks,
            retransmission_tx,
        );

        AcknowledgementController {
            ack_key,
            acknowledgement_listener: Some(acknowledgement_listener),
            input_message_listener: Some(input_message_listener),
            retransmission_request_listener: Some(retransmission_request_listener),
            sent_notification_listener: Some(sent_notification_listener),
        }
    }

    pub(super) fn ack_key(&self) -> Arc<AckAes128Key> {
        Arc::clone(&self.ack_key)
    }

    pub(super) async fn run(&mut self) {
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

    #[allow(dead_code)]
    pub(super) fn start(mut self) -> JoinHandle<Self> {
        tokio::spawn(async move {
            self.run().await;
            self
        })
    }
}
