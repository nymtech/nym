// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::helpers::new_interval_stream;
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::reply_controller::key_rotation_helpers::KeyRotationConfig;
use crate::client::replies::reply_storage::CombinedReplyStorage;
use crate::config;
use futures::StreamExt;
use nym_task::TaskClient;
use rand::rngs::OsRng;
use rand::{CryptoRng, Rng};
use std::time::Duration;
use time::OffsetDateTime;
use tracing::debug;

use crate::client::replies::reply_controller::receiver_controller::ReceiverReplyController;
use crate::client::replies::reply_controller::sender_controller::SenderReplyController;
pub(crate) use requests::{ReplyControllerMessage, ReplyControllerReceiver, ReplyControllerSender};

pub mod key_rotation_helpers;
mod receiver_controller;
pub mod requests;
mod sender_controller;

#[derive(Clone, Copy)]
pub struct Config {
    reply_surbs: config::ReplySurbs,

    /// Current configuration value of the key rotation as setup on this network.
    /// This includes things such as number of epochs per rotation, duration of epochs, etc.
    // NOTE: this is operating on the assumption of constant-length epochs
    key_rotation: KeyRotationConfig,
}

impl Config {
    pub(crate) fn new(
        reply_surbs_cfg: config::ReplySurbs,
        key_rotation: KeyRotationConfig,
    ) -> Self {
        Self {
            reply_surbs: reply_surbs_cfg,
            key_rotation,
        }
    }
}

// the purpose of this task:
// - buffers split messages from input message listener if there were insufficient surbs to send them
// - upon getting extra surbs, resends them
// - so I guess it will handle all 'RepliableMessage' and requests from 'ReplyMessage'
// - replies to "give additional surbs" requests
// - will reply to future heartbeats
pub type MaxRetransmissions = Option<u32>;

pub struct ReplyController<R> {
    config: Config,

    sender_controller: SenderReplyController<R>,
    receiver_controller: ReceiverReplyController<R>,

    request_receiver: ReplyControllerReceiver,

    // Listen for shutdown signals
    task_client: TaskClient,
}

impl ReplyController<OsRng> {
    pub(crate) fn new(
        config: Config,
        message_handler: MessageHandler<OsRng>,
        full_reply_storage: CombinedReplyStorage,
        request_receiver: ReplyControllerReceiver,
        task_client: TaskClient,
    ) -> Self {
        ReplyController {
            config,
            sender_controller: SenderReplyController::new(
                config,
                &full_reply_storage,
                message_handler.clone(),
            ),
            receiver_controller: ReceiverReplyController::new(
                config,
                full_reply_storage.surbs_storage(),
                message_handler,
            ),
            request_receiver,
            task_client,
        }
    }
}

impl<R> ReplyController<R>
where
    R: CryptoRng + Rng,
{
    async fn handle_request(&mut self, request: ReplyControllerMessage) {
        match request {
            ReplyControllerMessage::RetransmitReply {
                recipient,
                timed_out_ack,
                extra_surb_request,
            } => {
                self.receiver_controller
                    .handle_reply_retransmission(recipient, timed_out_ack, extra_surb_request)
                    .await
            }
            ReplyControllerMessage::SendReply {
                recipient,
                message,
                lane,
                max_retransmissions,
            } => {
                self.receiver_controller
                    .handle_send_reply(recipient, message, lane, max_retransmissions)
                    .await
            }
            ReplyControllerMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
                from_surb_request,
            } => {
                self.receiver_controller
                    .handle_received_surbs(sender_tag, reply_surbs, from_surb_request)
                    .await
            }
            ReplyControllerMessage::LaneQueueLength {
                connection_id,
                response_channel,
            } => self
                .receiver_controller
                .handle_lane_queue_length(connection_id, response_channel),
            ReplyControllerMessage::AdditionalSurbsRequest { recipient, amount } => {
                self.sender_controller
                    .handle_surb_request(*recipient, amount)
                    .await
            }
        }
    }

    async fn remove_stale_storage(&mut self) {
        let now = OffsetDateTime::now_utc();

        self.receiver_controller
            .inspect_and_clear_stale_data(now)
            .await;
        self.sender_controller.inspect_and_clear_stale_data(now)
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started ReplyController with graceful shutdown support");

        let mut shutdown = self.task_client.fork("reply-controller");

        let polling_rate = Duration::from_secs(5);
        let mut stale_inspection = new_interval_stream(polling_rate);

        let polling_rate = self.config.key_rotation.epoch_duration / 8;
        let mut invalidation_inspection = new_interval_stream(polling_rate);

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    tracing::trace!("ReplyController: Received shutdown");
                },
                req = self.request_receiver.next() => match req {
                    Some(req) => self.handle_request(req).await,
                    None => {
                        tracing::trace!("ReplyController: Stopping since channel closed");
                        break;
                    }
                },
                _ = stale_inspection.next() => {
                    self.receiver_controller.inspect_stale_pending_data().await
                },
                _ = invalidation_inspection.next() => {
                    self.receiver_controller.check_surb_refresh().await;
                    self.remove_stale_storage().await;
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        tracing::debug!("ReplyController: Exiting");
    }
}
