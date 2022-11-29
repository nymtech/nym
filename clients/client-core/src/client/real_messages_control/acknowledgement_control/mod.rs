// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::{
    acknowledgement_listener::AcknowledgementListener, action_controller::ActionController,
    input_message_listener::InputMessageListener,
    retransmission_request_listener::RetransmissionRequestListener,
    sent_notification_listener::SentNotificationListener,
};
use super::real_traffic_stream::BatchRealMessageSender;
use crate::client::{inbound_messages::InputMessageReceiver, topology_control::TopologyAccessor};
use crate::spawn_future;
use futures::channel::mpsc;
use gateway_client::AcknowledgementReceiver;
use log::*;
use nymsphinx::params::PacketSize;
use nymsphinx::{
    acknowledgements::AckKey,
    addressing::clients::Recipient,
    chunking::fragment::{Fragment, FragmentIdentifier},
    preparer::MessagePreparer,
    Delay as SphinxDelay,
};
use rand::{CryptoRng, Rng};
use std::{
    sync::{Arc, Weak},
    time::Duration,
};

#[cfg(feature = "reply-surb")]
use crate::client::reply_key_storage::ReplyKeyStorage;

mod acknowledgement_listener;
mod action_controller;
mod input_message_listener;
mod retransmission_request_listener;
mod sent_notification_listener;

/// Channel used for indicating that the particular `Fragment` should be retransmitted.
type RetransmissionRequestSender = mpsc::UnboundedSender<Weak<PendingAcknowledgement>>;

/// Channel used for receiving data about particular `Fragment` that should be retransmitted.
type RetransmissionRequestReceiver = mpsc::UnboundedReceiver<Weak<PendingAcknowledgement>>;

/// Channel used for signalling that the particular `Fragment` (associated with the `FragmentIdentifier`)
/// is done being delayed and is about to be sent to the mix network.
pub(super) type SentPacketNotificationSender = mpsc::UnboundedSender<FragmentIdentifier>;

/// Channel used for receiving signals about the particular `Fragment` (associated with the `FragmentIdentifier`)
/// that it is about to be sent to the mix network and its timeout timer should be started.
type SentPacketNotificationReceiver = mpsc::UnboundedReceiver<FragmentIdentifier>;

/// Structure representing a data `Fragment` that is on-route to the specified `Recipient`
#[derive(Debug)]
pub(crate) struct PendingAcknowledgement {
    message_chunk: Fragment,
    delay: SphinxDelay,
    recipient: Recipient,
}

impl PendingAcknowledgement {
    /// Creates new instance of `PendingAcknowledgement` using the provided data.
    fn new(message_chunk: Fragment, delay: SphinxDelay, recipient: Recipient) -> Self {
        PendingAcknowledgement {
            message_chunk,
            delay,
            recipient,
        }
    }

    fn update_delay(&mut self, new_delay: SphinxDelay) {
        self.delay = new_delay;
    }
}

/// AcknowledgementControllerConnectors represents set of channels for communication with
/// other parts of the system in order to support acknowledgements and retransmission.
pub(super) struct AcknowledgementControllerConnectors {
    /// Channel used for forwarding prepared sphinx messages into the poisson sender
    /// to be sent to the mix network.
    real_message_sender: BatchRealMessageSender,

    /// Channel used for receiving raw messages from a client. The messages need to be put
    /// into sphinx packets first.
    input_receiver: InputMessageReceiver,

    /// Channel used for receiving notification about particular packet being sent off to the
    /// mix network (i.e. it was done being delayed by whatever value was determined in the poisson
    /// sender)
    sent_notifier: SentPacketNotificationReceiver,

    /// Channel used for receiving acknowledgements from the mix network.
    ack_receiver: AcknowledgementReceiver,
}

impl AcknowledgementControllerConnectors {
    pub(super) fn new(
        real_message_sender: BatchRealMessageSender,
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

/// Configurable parameters of the `AcknowledgementController`
pub(super) struct Config {
    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the additive part `b`
    ack_wait_addition: Duration,

    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the multiplier `a`
    ack_wait_multiplier: f64,

    /// Average delay an acknowledgement packet is going to get delayed at a single mixnode.
    average_ack_delay: Duration,

    /// Average delay a data packet is going to get delayed at a single mixnode.
    average_packet_delay: Duration,

    /// Predefined packet size used for the encapsulated messages.
    packet_size: PacketSize,
}

impl Config {
    pub(super) fn new(
        ack_wait_addition: Duration,
        ack_wait_multiplier: f64,
        average_ack_delay: Duration,
        average_packet_delay: Duration,
    ) -> Self {
        Config {
            ack_wait_addition,
            ack_wait_multiplier,
            average_ack_delay,
            average_packet_delay,
            packet_size: Default::default(),
        }
    }

    pub fn with_custom_packet_size(mut self, packet_size: PacketSize) -> Self {
        self.packet_size = packet_size;
        self
    }
}

pub(super) struct AcknowledgementController<R>
where
    R: CryptoRng + Rng,
{
    acknowledgement_listener: AcknowledgementListener,
    input_message_listener: InputMessageListener<R>,
    retransmission_request_listener: RetransmissionRequestListener<R>,
    sent_notification_listener: SentNotificationListener,
    action_controller: ActionController,
}

impl<R> AcknowledgementController<R>
where
    R: 'static + CryptoRng + Rng + Clone + Send,
{
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        config: Config,
        rng: R,
        topology_access: TopologyAccessor,
        ack_key: Arc<AckKey>,
        ack_recipient: Recipient,
        connectors: AcknowledgementControllerConnectors,
        #[cfg(feature = "reply-surb")] reply_key_storage: ReplyKeyStorage,
    ) -> Self {
        let (retransmission_tx, retransmission_rx) = mpsc::unbounded();

        let action_config =
            action_controller::Config::new(config.ack_wait_addition, config.ack_wait_multiplier);
        let (action_controller, action_sender) =
            ActionController::new(action_config, retransmission_tx);

        let message_preparer = MessagePreparer::new(
            rng,
            ack_recipient,
            config.average_packet_delay,
            config.average_ack_delay,
        )
        .with_custom_real_message_packet_size(config.packet_size);

        // will listen for any acks coming from the network
        let acknowledgement_listener = AcknowledgementListener::new(
            Arc::clone(&ack_key),
            connectors.ack_receiver,
            action_sender.clone(),
        );

        // will listen for any new messages from the client
        let input_message_listener = InputMessageListener::new(
            Arc::clone(&ack_key),
            ack_recipient,
            connectors.input_receiver,
            message_preparer.clone(),
            action_sender.clone(),
            connectors.real_message_sender.clone(),
            topology_access.clone(),
            #[cfg(feature = "reply-surb")]
            reply_key_storage,
        );

        // will listen for any ack timeouts and trigger retransmission
        let retransmission_request_listener = RetransmissionRequestListener::new(
            Arc::clone(&ack_key),
            ack_recipient,
            message_preparer,
            action_sender.clone(),
            connectors.real_message_sender,
            retransmission_rx,
            topology_access,
        );

        // will listen for events indicating the packet was sent through the network so that
        // the retransmission timer should be started.
        let sent_notification_listener =
            SentNotificationListener::new(connectors.sent_notifier, action_sender);

        AcknowledgementController {
            acknowledgement_listener,
            input_message_listener,
            retransmission_request_listener,
            sent_notification_listener,
            action_controller,
        }
    }

    pub(super) fn start_with_shutdown(self, shutdown: task::ShutdownListener) {
        let mut acknowledgement_listener = self.acknowledgement_listener;
        let mut input_message_listener = self.input_message_listener;
        let mut retransmission_request_listener = self.retransmission_request_listener;
        let mut sent_notification_listener = self.sent_notification_listener;
        let mut action_controller = self.action_controller;

        let shutdown_handle = shutdown.clone();
        spawn_future(async move {
            acknowledgement_listener
                .run_with_shutdown(shutdown_handle)
                .await;
            debug!("The acknowledgement listener has finished execution!");
        });

        let shutdown_handle = shutdown.clone();
        spawn_future(async move {
            input_message_listener
                .run_with_shutdown(shutdown_handle)
                .await;
            debug!("The input listener has finished execution!");
        });

        let shutdown_handle = shutdown.clone();
        spawn_future(async move {
            retransmission_request_listener
                .run_with_shutdown(shutdown_handle)
                .await;
            debug!("The retransmission request listener has finished execution!");
        });

        let shutdown_handle = shutdown.clone();
        spawn_future(async move {
            sent_notification_listener
                .run_with_shutdown(shutdown_handle)
                .await;
            debug!("The sent notification listener has finished execution!");
        });

        spawn_future(async move {
            action_controller.run_with_shutdown(shutdown).await;
            debug!("The controller has finished execution!");
        });
    }

    // todo: think whether this is still required
    #[allow(dead_code)]
    pub(super) fn start(self) {
        let mut acknowledgement_listener = self.acknowledgement_listener;
        let mut input_message_listener = self.input_message_listener;
        let mut retransmission_request_listener = self.retransmission_request_listener;
        let mut sent_notification_listener = self.sent_notification_listener;
        let mut action_controller = self.action_controller;

        spawn_future(async move {
            acknowledgement_listener.run().await;
            error!("The acknowledgement listener has finished execution!");
        });
        spawn_future(async move {
            input_message_listener.run().await;
            error!("The input listener has finished execution!");
        });
        spawn_future(async move {
            retransmission_request_listener.run().await;
            error!("The retransmission request listener has finished execution!");
        });
        spawn_future(async move {
            sent_notification_listener.run().await;
            error!("The sent notification listener has finished execution!");
        });
        spawn_future(async move {
            action_controller.run().await;
            error!("The controller has finished execution!");
        });
    }
}
