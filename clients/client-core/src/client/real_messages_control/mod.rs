// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// INPUT: InputMessage from user
// INPUT2: Acks from mix
// OUTPUT: MixMessage to mix traffic

use self::{
    acknowledgement_control::AcknowledgementController, real_traffic_stream::OutQueueControl,
};
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::reply_controller::{
    ReplyController, ReplyControllerReceiver, ReplyControllerSender,
};
use crate::client::replies::reply_storage::CombinedReplyStorage;
use crate::{
    client::{
        inbound_messages::InputMessageReceiver, mix_traffic::BatchMixMessageSender,
        real_messages_control::acknowledgement_control::AcknowledgementControllerConnectors,
        topology_control::TopologyAccessor,
    },
    spawn_future,
};
use client_connections::{ConnectionCommandReceiver, LaneQueueLengths};
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

use crate::config;
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

    /// Defines the minimum number of reply surbs the client would request.
    minimum_reply_surb_request_size: u32,

    /// Defines the maximum number of reply surbs the client would request.
    maximum_reply_surb_request_size: u32,

    /// Defines the maximum number of reply surbs a remote party is allowed to request from this client at once.
    maximum_allowed_reply_surb_request_size: u32,

    /// Defines the amount of reply surbs that the client is going to request when it runs out while attempting to retransmit packets.
    retransmission_reply_surb_request_size: u32,

    /// Defines maximum amount of time the client is going to wait for reply surbs before explicitly asking
    /// for more even though in theory they wouldn't need to.
    maximum_reply_surb_waiting_period: Duration,
}

impl Config {
    pub fn new(
        base_client_debug_config: &config::Debug,
        ack_key: Arc<AckKey>,
        self_recipient: Recipient,
    ) -> Self {
        Config {
            ack_key,
            self_recipient,
            packet_size: Default::default(),
            ack_wait_addition: base_client_debug_config.ack_wait_addition,
            ack_wait_multiplier: base_client_debug_config.ack_wait_multiplier,
            average_message_sending_delay: base_client_debug_config.message_sending_average_delay,
            average_packet_delay_duration: base_client_debug_config.average_packet_delay,
            average_ack_delay_duration: base_client_debug_config.average_ack_delay,
            disable_main_poisson_packet_distribution: base_client_debug_config
                .disable_main_poisson_packet_distribution,
            minimum_reply_surb_request_size: base_client_debug_config
                .minimum_reply_surb_request_size,
            maximum_reply_surb_request_size: base_client_debug_config
                .maximum_reply_surb_request_size,
            maximum_allowed_reply_surb_request_size: base_client_debug_config
                .maximum_allowed_reply_surb_request_size,
            retransmission_reply_surb_request_size: base_client_debug_config
                .retransmission_reply_surb_request_size,
            maximum_reply_surb_waiting_period: base_client_debug_config
                .maximum_reply_surb_waiting_period,
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
    reply_control: ReplyController<R>,
}

// obviously when we finally make shared rng that is on 'higher' level, this should become
// generic `R`
impl RealMessagesController<OsRng> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Config,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
        topology_access: TopologyAccessor,
        reply_storage: CombinedReplyStorage,
        // so much refactoring needed, but this is temporary just to test things out
        reply_controller_sender: ReplyControllerSender,
        reply_controller_receiver: ReplyControllerReceiver,
        lane_queue_lengths: LaneQueueLengths,
        client_connection_rx: ConnectionCommandReceiver,
    ) -> Self {
        let rng = OsRng;

        let (real_message_sender, real_message_receiver) = tokio::sync::mpsc::channel(1);
        let (sent_notifier_tx, sent_notifier_rx) = mpsc::unbounded();
        let (ack_action_tx, ack_action_rx) = mpsc::unbounded();

        let ack_controller_connectors = AcknowledgementControllerConnectors::new(
            input_receiver,
            sent_notifier_rx,
            ack_receiver,
            ack_action_tx.clone(),
            ack_action_rx,
        );

        let ack_control_config = acknowledgement_control::Config::new(
            config.ack_wait_addition,
            config.ack_wait_multiplier,
            config.retransmission_reply_surb_request_size,
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
            rng,
            Arc::clone(&config.ack_key),
            config.self_recipient,
            message_preparer,
            ack_action_tx,
            real_message_sender,
            topology_access.clone(),
            reply_storage.key_storage(),
            reply_storage.tags_storage(),
        );

        let reply_control = ReplyController::new(
            message_handler.clone(),
            reply_storage.surbs_storage(),
            reply_storage.tags_storage(),
            reply_controller_receiver,
            config.minimum_reply_surb_request_size,
            config.maximum_reply_surb_request_size,
            config.maximum_allowed_reply_surb_request_size,
            config.maximum_reply_surb_waiting_period,
        );

        let ack_control = AcknowledgementController::new(
            ack_control_config,
            Arc::clone(&config.ack_key),
            ack_controller_connectors,
            message_handler,
            reply_controller_sender,
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
            topology_access,
            lane_queue_lengths,
            client_connection_rx,
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
