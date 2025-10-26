// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// INPUT: InputMessage from user
// INPUT2: Acks from mix
// OUTPUT: MixMessage to mix traffic

use self::{
    acknowledgement_control::AcknowledgementController, real_traffic_stream::OutQueueControl,
};
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::reply_controller;
use crate::client::replies::reply_controller::{
    ReplyController, ReplyControllerReceiver, ReplyControllerSender,
};
use crate::client::replies::reply_storage::CombinedReplyStorage;
use crate::client::{
    inbound_messages::InputMessageReceiver, mix_traffic::BatchMixMessageSender,
    real_messages_control::acknowledgement_control::AcknowledgementControllerConnectors,
    topology_control::TopologyAccessor,
};
use crate::config;
use futures::channel::mpsc;
use nym_gateway_client::AcknowledgementReceiver;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_statistics_common::clients::ClientStatsSender;
use nym_task::connections::{ConnectionCommandReceiver, LaneQueueLengths};
use nym_task::ShutdownToken;
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::sync::Arc;

use crate::client::replies::reply_controller::key_rotation_helpers::KeyRotationConfig;
pub(crate) use acknowledgement_control::{AckActionSender, Action};

pub(crate) mod acknowledgement_control;
pub(crate) mod message_handler;
pub(crate) mod real_traffic_stream;

// TODO: ack_key and self_recipient shouldn't really be part of this config
pub struct Config {
    /// Key used to decrypt contents of received SURBAcks
    ack_key: Arc<AckKey>,

    /// Address of `this` client.
    self_recipient: Recipient,

    /// Specifies all traffic related configuration options.
    traffic: config::Traffic,

    /// Specifies all cover traffic related configuration options.
    cover_traffic: config::CoverTraffic,

    /// Specifies all acknowledgements related configuration options.
    acks: config::Acknowledgements,

    /// Specifies all reply SURBs related configuration options.
    reply_surbs: config::ReplySurbs,
}

impl<'a> From<&'a Config> for acknowledgement_control::Config {
    fn from(cfg: &'a Config) -> Self {
        acknowledgement_control::Config::new(
            cfg.traffic.maximum_number_of_retransmissions,
            cfg.acks.ack_wait_addition,
            cfg.acks.ack_wait_multiplier,
            cfg.traffic.packet_type,
        )
        .with_custom_packet_size(cfg.traffic.primary_packet_size)
    }
}

impl<'a> From<&'a Config> for real_traffic_stream::Config {
    fn from(cfg: &'a Config) -> Self {
        real_traffic_stream::Config::new(
            Arc::clone(&cfg.ack_key),
            cfg.self_recipient,
            cfg.acks.average_ack_delay,
            cfg.traffic,
            cfg.cover_traffic.cover_traffic_primary_size_ratio,
        )
    }
}

impl<'a> From<&'a Config> for message_handler::Config {
    fn from(cfg: &'a Config) -> Self {
        message_handler::Config::new(
            Arc::clone(&cfg.ack_key),
            cfg.self_recipient,
            cfg.traffic.average_packet_delay,
            cfg.acks.average_ack_delay,
            cfg.traffic.deterministic_route_selection,
            cfg.traffic.use_legacy_sphinx_format,
        )
        .with_custom_primary_packet_size(cfg.traffic.primary_packet_size)
        .with_custom_secondary_packet_size(cfg.traffic.secondary_packet_size)
        .disable_mix_hops(cfg.traffic.disable_mix_hops)
    }
}

impl Config {
    pub fn new(
        base_client_debug_config: &config::DebugConfig,
        ack_key: Arc<AckKey>,
        self_recipient: Recipient,
    ) -> Self {
        Config {
            ack_key,
            self_recipient,
            traffic: base_client_debug_config.traffic,
            cover_traffic: base_client_debug_config.cover_traffic,
            acks: base_client_debug_config.acknowledgements,
            reply_surbs: base_client_debug_config.reply_surbs,
        }
    }
}

pub(crate) struct RealMessagesController<R>
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
    pub(crate) fn new(
        config: Config,
        key_rotation_config: KeyRotationConfig,
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
        stats_tx: ClientStatsSender,
        shutdown_token: ShutdownToken,
    ) -> Self {
        let rng = OsRng;

        // create channels for inter-task communication
        let (real_message_sender, real_message_receiver) = tokio::sync::mpsc::channel(8);
        let (sent_notifier_tx, sent_notifier_rx) = mpsc::unbounded();
        let (ack_action_tx, ack_action_rx) = mpsc::unbounded();
        let ack_controller_connectors = AcknowledgementControllerConnectors::new(
            input_receiver,
            sent_notifier_rx,
            ack_receiver,
            ack_action_tx.clone(),
            ack_action_rx,
        );

        // create all configs for the components
        let ack_control_config = (&config).into();
        let out_queue_config = (&config).into();
        let reply_controller_config =
            reply_controller::Config::new(config.reply_surbs, key_rotation_config);
        let message_handler_config = (&config).into();

        // create the actual components
        let message_handler = MessageHandler::new(
            message_handler_config,
            rng,
            ack_action_tx,
            real_message_sender,
            topology_access.clone(),
            reply_storage.key_storage(),
            reply_storage.tags_storage(),
            shutdown_token.clone(),
        );

        let ack_control = AcknowledgementController::new(
            ack_control_config,
            Arc::clone(&config.ack_key),
            ack_controller_connectors,
            message_handler.clone(),
            reply_controller_sender,
            stats_tx.clone(),
        );

        let reply_control = ReplyController::new(
            reply_controller_config,
            message_handler,
            reply_storage,
            reply_controller_receiver,
        );

        let out_queue_control = OutQueueControl::new(
            out_queue_config,
            rng,
            sent_notifier_tx,
            mix_sender,
            real_message_receiver,
            topology_access,
            lane_queue_lengths,
            client_connection_rx,
            stats_tx,
            shutdown_token.clone(),
        );
println!("average packet delay {:?} message sending delay {:?}",config.traffic.average_packet_delay,config.traffic.message_sending_average_delay);
        RealMessagesController {
            out_queue_control,
            ack_control,
            reply_control,
        }
    }

    pub fn into_tasks(
        self,
    ) -> (
        OutQueueControl<OsRng>,
        ReplyController<OsRng>,
        AcknowledgementController<OsRng>,
    ) {
        (self.out_queue_control, self.reply_control, self.ack_control)
    }
}
