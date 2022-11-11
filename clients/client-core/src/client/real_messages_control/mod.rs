// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// INPUT: InputMessage from user
// INPUT2: Acks from mix
// OUTPUT: MixMessage to mix traffic

use self::{
    acknowledgement_control::AcknowledgementController, real_traffic_stream::OutQueueControl,
};
use crate::client::real_messages_control::acknowledgement_control::AcknowledgementControllerConnectors;
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::reply_storage::{CombinedReplyStorage, SentReplyKeys};
use crate::client::replies::temp_name_pending_handler::{
    ToBeNamedPendingReplyController, ToBeNamedReceiver, ToBeNamedSender,
};
use crate::client::{
    inbound_messages::InputMessageReceiver, mix_traffic::BatchMixMessageSender,
    topology_control::TopologyAccessor,
};
use crate::spawn_future;
use futures::channel::mpsc;
use gateway_client::AcknowledgementReceiver;
use log::*;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::params::PacketSize;
use nymsphinx::preparer::MessagePreparer;
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::sync::Arc;
use std::time::Duration;

pub(crate) use acknowledgement_control::{AckActionSender, Action};
// #[cfg(feature = "reply-surb")]
// use crate::client::reply_key_storage::ReplyKeyStorage;

pub(crate) mod acknowledgement_control;
pub(crate) mod message_handler;
pub(crate) mod real_traffic_stream;

// TODO: ack_key and self_recipient shouldn't really be part of this config
pub struct Config {
    /// Key used to decrypt contents of received SURBAcks
    ack_key: Arc<AckKey>,

    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the additive part `b`
    ack_wait_addition: Duration,

    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the multiplier `a`
    ack_wait_multiplier: f64,

    /// Address of `this` client.
    self_recipient: Recipient,

    /// Average delay between sending subsequent packets from this client.
    average_message_sending_delay: Duration,

    /// Average delay a data packet is going to get delayed at a single mixnode.
    average_packet_delay_duration: Duration,

    /// Average delay an acknowledgement packet is going to get delayed at a single mixnode.
    average_ack_delay_duration: Duration,

    /// Controls whether the main packet stream constantly produces packets according to the predefined
    /// poisson distribution.
    disable_main_poisson_packet_distribution: bool,

    /// Predefined packet size used for the encapsulated messages.
    packet_size: PacketSize,
}

impl Config {
    // TODO: change the config into a builder
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ack_key: Arc<AckKey>,
        ack_wait_multiplier: f64,
        ack_wait_addition: Duration,
        average_ack_delay_duration: Duration,
        average_message_sending_delay: Duration,
        average_packet_delay_duration: Duration,
        disable_main_poisson_packet_distribution: bool,
        self_recipient: Recipient,
    ) -> Self {
        Config {
            ack_key,
            ack_wait_addition,
            ack_wait_multiplier,
            self_recipient,
            average_message_sending_delay,
            average_packet_delay_duration,
            average_ack_delay_duration,
            disable_main_poisson_packet_distribution,
            packet_size: Default::default(),
        }
    }

    pub fn set_custom_packet_size(&mut self, packet_size: PacketSize) {
        self.packet_size = packet_size;
    }
}

pub struct RealMessagesController<R>
where
    R: CryptoRng + Rng,
{
    out_queue_control: OutQueueControl<R>,
    ack_control: AcknowledgementController<R>,
    reply_control: ToBeNamedPendingReplyController<R>,
}

// obviously when we finally make shared rng that is on 'higher' level, this should become
// generic `R`
impl RealMessagesController<OsRng> {
    pub fn new(
        config: Config,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
        topology_access: TopologyAccessor,
        reply_storage: CombinedReplyStorage,
        // so much refactoring needed, but this is temporary just to test things out
        to_be_named_channel_sender: ToBeNamedSender,
        to_be_named_channel_receiver: ToBeNamedReceiver,
    ) -> Self {
        let rng = OsRng;

        let (real_message_sender, real_message_receiver) = mpsc::unbounded();
        let (sent_notifier_tx, sent_notifier_rx) = mpsc::unbounded();
        let (ack_action_tx, ack_action_rx) = mpsc::unbounded();

        let ack_controller_connectors = AcknowledgementControllerConnectors::new(
            real_message_sender.clone(),
            input_receiver,
            sent_notifier_rx,
            ack_receiver,
            ack_action_tx.clone(),
            ack_action_rx,
        );

        let ack_control_config = acknowledgement_control::Config::new(
            config.ack_wait_addition,
            config.ack_wait_multiplier,
            config.average_ack_delay_duration,
            config.average_packet_delay_duration,
        )
        .with_custom_packet_size(config.packet_size);

        // TODO: construct MessagePreparer itself inside the MessageHandler
        let message_preparer = MessagePreparer::new(
            rng,
            config.self_recipient,
            config.average_packet_delay_duration,
            config.average_ack_delay_duration,
        )
        .with_custom_real_message_packet_size(config.packet_size);
        let message_handler = MessageHandler::new(
            Arc::clone(&config.ack_key),
            config.self_recipient,
            message_preparer,
            ack_action_tx.clone(),
            real_message_sender.clone(),
            topology_access.clone(),
            reply_storage.key_storage(),
        );

        let reply_control = ToBeNamedPendingReplyController::new(
            message_handler.clone(),
            reply_storage.surbs_storage(),
            to_be_named_channel_receiver,
        );

        let ack_control = AcknowledgementController::new(
            ack_control_config,
            Arc::clone(&config.ack_key),
            ack_controller_connectors,
            message_handler,
            to_be_named_channel_sender,
            reply_storage.surbs_storage(),
        );

        let out_queue_config = real_traffic_stream::Config::new(
            config.average_ack_delay_duration,
            config.average_packet_delay_duration,
            config.average_message_sending_delay,
            config.disable_main_poisson_packet_distribution,
        )
        .with_custom_cover_packet_size(config.packet_size);

        let out_queue_control = OutQueueControl::new(
            out_queue_config,
            config.ack_key,
            sent_notifier_tx,
            mix_sender,
            real_message_receiver,
            rng,
            config.self_recipient,
            topology_access.clone(),
        );

        RealMessagesController {
            out_queue_control,
            ack_control,
            reply_control,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_with_shutdown(self, shutdown: task::ShutdownListener) {
        let mut out_queue_control = self.out_queue_control;
        let ack_control = self.ack_control;
        let mut reply_control = self.reply_control;

        let shutdown_handle = shutdown.clone();
        spawn_future(async move {
            out_queue_control.run_with_shutdown(shutdown_handle).await;
            debug!("The out queue controller has finished execution!");
        });
        let shutdown_handle = shutdown.clone();
        spawn_future(async move {
            reply_control.run_with_shutdown(shutdown_handle).await;
            debug!("The reply controller has finished execution!");
        });

        ack_control.start_with_shutdown(shutdown);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start(self) {
        let mut out_queue_control = self.out_queue_control;
        let ack_control = self.ack_control;
        let mut reply_control = self.reply_control;

        spawn_future(async move {
            out_queue_control.run().await;
            debug!("The out queue controller has finished execution!");
        });
        spawn_future(async move {
            reply_control.run().await;
            debug!("The reply controller has finished execution!");
        });
        ack_control.start();
    }
}
