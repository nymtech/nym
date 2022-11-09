// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// INPUT: InputMessage from user
// INPUT2: Acks from mix
// OUTPUT: MixMessage to mix traffic

use self::acknowledgement_control::action_controller::ActionSender;
use self::acknowledgement_control::input_message_listener::FreshInputMessageChunker;
use self::{
    acknowledgement_control::AcknowledgementController, real_traffic_stream::OutQueueControl,
};
use crate::client::real_messages_control::acknowledgement_control::AcknowledgementControllerConnectors;
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
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "reply-surb")]
use crate::client::reply_key_storage::ReplyKeyStorage;

pub mod acknowledgement_control;
pub mod real_traffic_stream;

// TODO: ack_key and self_recipient shouldn't really be part of this config
pub struct Config {
    /// Key used to decrypt contents of received SURBAcks
    ack_key: Arc<AckKey>,

    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the additive part `b`
    pub ack_wait_addition: Duration,

    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the multiplier `a`
    pub ack_wait_multiplier: f64,

    /// Address of `this` client.
    self_recipient: Recipient,

    /// Average delay between sending subsequent packets from this client.
    average_message_sending_delay: Duration,

    /// Average delay a data packet is going to get delayed at a single mixnode.
    pub average_packet_delay_duration: Duration,

    /// Average delay an acknowledgement packet is going to get delayed at a single mixnode.
    pub average_ack_delay_duration: Duration,

    /// Controls whether the main packet stream constantly produces packets according to the predefined
    /// poisson distribution.
    disable_main_poisson_packet_distribution: bool,

    /// Predefined packet size used for the encapsulated messages.
    pub packet_size: PacketSize,
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
}

// obviously when we finally make shared rng that is on 'higher' level, this should become
// generic `R`
impl RealMessagesController<OsRng> {
    pub fn get_action_sender(&self) -> ActionSender {
        self.ack_control.get_action_sender()
    }

    pub fn get_fresh_input_message_chunker(&self) -> FreshInputMessageChunker<OsRng> {
        self.ack_control.fresh_input_msg_chunker.clone()
    }

    pub fn new(
        config: Config,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
        topology_access: TopologyAccessor,
        #[cfg(feature = "reply-surb")] reply_key_storage: ReplyKeyStorage,
    ) -> Self {
        let rng = OsRng;

        let (real_message_sender, real_message_receiver) = tokio::sync::mpsc::channel(3);
        let (sent_notifier_tx, sent_notifier_rx) = mpsc::unbounded();

        let ack_controller_connectors = AcknowledgementControllerConnectors::new(
            real_message_sender,
            input_receiver,
            sent_notifier_rx,
            ack_receiver,
        );

        let ack_control_config = acknowledgement_control::Config::new(
            config.ack_wait_addition,
            config.ack_wait_multiplier,
            config.average_ack_delay_duration,
            config.average_packet_delay_duration,
        )
        .with_custom_packet_size(config.packet_size);

        let ack_control = AcknowledgementController::new(
            ack_control_config,
            rng,
            topology_access.clone(),
            Arc::clone(&config.ack_key),
            config.self_recipient,
            ack_controller_connectors,
            #[cfg(feature = "reply-surb")]
            reply_key_storage,
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
            Arc::clone(&config.ack_key),
            sent_notifier_tx,
            mix_sender,
            real_message_receiver,
            rng,
            config.self_recipient,
            topology_access,
        );

        RealMessagesController {
            out_queue_control,
            ack_control,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_with_shutdown(self, shutdown: task::ShutdownListener) {
        let mut out_queue_control = self.out_queue_control;
        let ack_control = self.ack_control;

        let shutdown_handle = shutdown.clone();
        spawn_future(async move {
            out_queue_control.run_with_shutdown(shutdown_handle).await;
            debug!("The out queue controller has finished execution!");
        });
        ack_control.start_with_shutdown(shutdown);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start(self) {
        let mut out_queue_control = self.out_queue_control;
        let ack_control = self.ack_control;

        spawn_future(async move {
            out_queue_control.run().await;
            debug!("The out queue controller has finished execution!");
        });
        ack_control.start();
    }
}
