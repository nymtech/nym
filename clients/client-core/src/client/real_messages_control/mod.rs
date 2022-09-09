// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// INPUT: InputMessage from user
// INPUT2: Acks from mix
// OUTPUT: MixMessage to mix traffic

use self::{
    acknowledgement_control::AcknowledgementController, real_traffic_stream::OutQueueControl,
};
use crate::client::real_messages_control::acknowledgement_control::AcknowledgementControllerConnectors;
use crate::client::reply_key_storage::ReplyKeyStorage;
use crate::client::{
    inbound_messages::InputMessageReceiver, mix_traffic::BatchMixMessageSender,
    topology_control::TopologyAccessor,
};
use futures::channel::mpsc;
use gateway_client::AcknowledgementReceiver;
use log::*;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::sync::Arc;
use std::time::Duration;
use task::ShutdownListener;
use tokio::task::JoinHandle;

mod acknowledgement_control;
mod real_traffic_stream;

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
}

impl Config {
    pub fn new(
        ack_key: Arc<AckKey>,
        ack_wait_multiplier: f64,
        ack_wait_addition: Duration,
        average_ack_delay_duration: Duration,
        average_message_sending_delay: Duration,
        average_packet_delay_duration: Duration,
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
        }
    }
}

pub struct RealMessagesController<R>
where
    R: CryptoRng + Rng,
{
    out_queue_control: Option<OutQueueControl<R>>,
    ack_control: Option<AcknowledgementController<R>>,
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
        reply_key_storage: ReplyKeyStorage,
        shutdown: ShutdownListener,
    ) -> Self {
        let rng = OsRng;

        let (real_message_sender, real_message_receiver) = mpsc::unbounded();
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
        );

        let ack_control = AcknowledgementController::new(
            ack_control_config,
            rng,
            topology_access.clone(),
            Arc::clone(&config.ack_key),
            config.self_recipient,
            reply_key_storage,
            ack_controller_connectors,
            shutdown.clone(),
        );

        let out_queue_config = real_traffic_stream::Config::new(
            config.average_ack_delay_duration,
            config.average_packet_delay_duration,
            config.average_message_sending_delay,
        );

        let out_queue_control = OutQueueControl::new(
            out_queue_config,
            Arc::clone(&config.ack_key),
            sent_notifier_tx,
            mix_sender,
            real_message_receiver,
            rng,
            config.self_recipient,
            topology_access,
            shutdown,
        );

        RealMessagesController {
            out_queue_control: Some(out_queue_control),
            ack_control: Some(ack_control),
        }
    }

    pub(super) async fn run(&mut self) {
        let mut out_queue_control = self.out_queue_control.take().unwrap();
        let mut ack_control = self.ack_control.take().unwrap();

        // the below are log messages are errors as at the current stage we do not expect any of
        // the task to ever finish. This will of course change once we introduce
        // graceful shutdowns.
        let out_queue_control_fut = tokio::spawn(async move {
            out_queue_control.run_out_queue_control().await;
            debug!("The out queue controller has finished execution!");
            out_queue_control
        });
        let ack_control_fut = tokio::spawn(async move {
            ack_control.run().await;
            debug!("The acknowledgement controller has finished execution!");
            ack_control
        });

        // technically we don't have to bring `RealMessagesController` back to a valid state
        // but we can do it, so why not? Perhaps it might be useful if we wanted to allow
        // for restarts of certain modules without killing the entire process.
        self.out_queue_control = Some(out_queue_control_fut.await.unwrap());
        self.ack_control = Some(ack_control_fut.await.unwrap());
    }

    pub fn start(mut self) -> JoinHandle<Self> {
        tokio::spawn(async move {
            self.run().await;
            self
        })
    }
}
