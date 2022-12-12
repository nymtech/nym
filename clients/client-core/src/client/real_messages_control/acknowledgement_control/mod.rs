// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::{
    acknowledgement_listener::AcknowledgementListener, action_controller::ActionController,
    input_message_listener::InputMessageListener,
    retransmission_request_listener::RetransmissionRequestListener,
    sent_notification_listener::SentNotificationListener,
};
use crate::client::inbound_messages::InputMessageReceiver;
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::reply_controller::ReplyControllerSender;
use crate::spawn_future;
use action_controller::AckActionReceiver;
use futures::channel::mpsc;
use gateway_client::AcknowledgementReceiver;
use log::*;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::params::PacketSize;
use nymsphinx::{
    acknowledgements::AckKey,
    addressing::clients::Recipient,
    chunking::fragment::{Fragment, FragmentIdentifier},
    Delay as SphinxDelay,
};
use rand::{CryptoRng, Rng};
use std::{
    sync::{Arc, Weak},
    time::Duration,
};

use crate::client::replies::reply_storage::ReceivedReplySurbsMap;
pub(crate) use action_controller::{AckActionSender, Action};

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

#[derive(Debug)]
pub(crate) enum PacketDestination {
    Anonymous {
        recipient_tag: AnonymousSenderTag,
        // special flag to indicate whether this was an ack for requesting additional surbs,
        // in that case we have to do everything we can to get it through, even if it means going
        // below our stored reply surb threshold
        extra_surb_request: bool,
    },
    KnownRecipient(Box<Recipient>),
}

/// Structure representing a data `Fragment` that is on-route to the specified `Recipient`
#[derive(Debug)]
pub(crate) struct PendingAcknowledgement {
    message_chunk: Fragment,
    delay: SphinxDelay,
    destination: PacketDestination,
}

impl PendingAcknowledgement {
    /// Creates new instance of `PendingAcknowledgement` using the provided data.
    pub(crate) fn new_known(
        message_chunk: Fragment,
        delay: SphinxDelay,
        recipient: Recipient,
    ) -> Self {
        PendingAcknowledgement {
            message_chunk,
            delay,
            destination: PacketDestination::KnownRecipient(recipient.into()),
        }
    }

    pub(crate) fn new_anonymous(
        message_chunk: Fragment,
        delay: SphinxDelay,
        recipient_tag: AnonymousSenderTag,
        extra_surb_request: bool,
    ) -> Self {
        PendingAcknowledgement {
            message_chunk,
            delay,
            destination: PacketDestination::Anonymous {
                recipient_tag,
                extra_surb_request,
            },
        }
    }

    pub(crate) fn is_extra_surb_request(&self) -> bool {
        if let PacketDestination::Anonymous {
            extra_surb_request, ..
        } = self.destination
        {
            extra_surb_request
        } else {
            false
        }
    }

    pub(crate) fn inner_fragment_identifier(&self) -> FragmentIdentifier {
        self.message_chunk.fragment_identifier()
    }

    pub(crate) fn fragment_data(&self) -> Fragment {
        self.message_chunk.clone()
    }

    fn update_delay(&mut self, new_delay: SphinxDelay) {
        self.delay = new_delay;
    }
}

/// AcknowledgementControllerConnectors represents set of channels for communication with
/// other parts of the system in order to support acknowledgements and retransmission.
pub(super) struct AcknowledgementControllerConnectors {
    /// Channel used for receiving raw messages from a client. The messages need to be put
    /// into sphinx packets first.
    input_receiver: InputMessageReceiver,

    /// Channel used for receiving notification about particular packet being sent off to the
    /// mix network (i.e. it was done being delayed by whatever value was determined in the poisson
    /// sender)
    sent_notifier: SentPacketNotificationReceiver,

    /// Channel used for receiving acknowledgements from the mix network.
    ack_receiver: AcknowledgementReceiver,

    /// Channel used for sending request to `ActionController` to deal with anything ack-related,
    ack_action_sender: AckActionSender,

    /// Channel used for receiving request by `ActionController` to deal with anything ack-related,
    ack_action_receiver: AckActionReceiver,
}

impl AcknowledgementControllerConnectors {
    pub(super) fn new(
        input_receiver: InputMessageReceiver,
        sent_notifier: SentPacketNotificationReceiver,
        ack_receiver: AcknowledgementReceiver,
        ack_action_sender: AckActionSender,
        ack_action_receiver: AckActionReceiver,
    ) -> Self {
        AcknowledgementControllerConnectors {
            input_receiver,
            sent_notifier,
            ack_receiver,
            ack_action_sender,
            ack_action_receiver,
        }
    }
}

/// Configurable parameters of the `AcknowledgementController`
pub(super) struct Config {
    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the additive part `b`
    ack_wait_addition: Duration,

    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the multiplier `a`
    ack_wait_multiplier: f64,

    /// Defines the amount of reply surbs that the client is going to request when it runs out while attempting to retransmit packets.
    retransmission_reply_surb_request_size: u32,

    /// Predefined packet size used for the encapsulated messages.
    packet_size: PacketSize,
}

impl Config {
    pub(super) fn new(
        ack_wait_addition: Duration,
        ack_wait_multiplier: f64,
        retransmission_reply_surb_request_size: u32,
    ) -> Self {
        Config {
            ack_wait_addition,
            ack_wait_multiplier,
            retransmission_reply_surb_request_size,
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
    R: 'static + CryptoRng + Rng + Clone + Send + Sync,
{
    pub(super) fn new(
        config: Config,
        ack_key: Arc<AckKey>,
        connectors: AcknowledgementControllerConnectors,
        message_handler: MessageHandler<R>,
        reply_controller_sender: ReplyControllerSender,
        received_reply_surbs: ReceivedReplySurbsMap,
    ) -> Self {
        let (retransmission_tx, retransmission_rx) = mpsc::unbounded();

        let action_config =
            action_controller::Config::new(config.ack_wait_addition, config.ack_wait_multiplier);
        let action_controller = ActionController::new(
            action_config,
            retransmission_tx,
            connectors.ack_action_receiver,
        );

        // will listen for any acks coming from the network
        let acknowledgement_listener = AcknowledgementListener::new(
            Arc::clone(&ack_key),
            connectors.ack_receiver,
            connectors.ack_action_sender.clone(),
        );

        // will listen for any new messages from the client
        let input_message_listener = InputMessageListener::new(
            connectors.input_receiver,
            message_handler.clone(),
            reply_controller_sender.clone(),
        );

        // will listen for any ack timeouts and trigger retransmission
        let retransmission_request_listener = RetransmissionRequestListener::new(
            connectors.ack_action_sender.clone(),
            message_handler,
            retransmission_rx,
            reply_controller_sender,
            received_reply_surbs,
            config.retransmission_reply_surb_request_size,
        );

        // will listen for events indicating the packet was sent through the network so that
        // the retransmission timer should be started.
        let sent_notification_listener =
            SentNotificationListener::new(connectors.sent_notifier, connectors.ack_action_sender);

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
}
