// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::inbound_messages::{InputMessage, InputMessageReceiver};
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::reply_controller::ReplyControllerSender;
use log::*;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_task::connections::TransmissionLane;
use rand::{CryptoRng, Rng};

/// Module responsible for dealing with the received messages: splitting them, creating acknowledgements,
/// putting everything into sphinx packets, etc.
/// It also makes an initial sending attempt for said messages.
pub(super) struct InputMessageListener<R>
where
    R: CryptoRng + Rng,
{
    input_receiver: InputMessageReceiver,
    message_handler: MessageHandler<R>,
    reply_controller_sender: ReplyControllerSender,
}

impl<R> InputMessageListener<R>
where
    R: CryptoRng + Rng,
{
    // at this point I'm not entirely sure how to deal with this warning without
    // some considerable refactoring
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        input_receiver: InputMessageReceiver,
        message_handler: MessageHandler<R>,
        reply_controller_sender: ReplyControllerSender,
    ) -> Self {
        InputMessageListener {
            input_receiver,
            message_handler,
            reply_controller_sender,
        }
    }

    async fn handle_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
    ) {
        // offload reply handling to the dedicated task
        self.reply_controller_sender
            .send_reply(recipient_tag, data, lane)
    }

    async fn handle_plain_message(
        &mut self,
        recipient: Recipient,
        content: Vec<u8>,
        lane: TransmissionLane,
    ) {
        if let Err(err) = self
            .message_handler
            .try_send_plain_message(recipient, content, lane)
            .await
        {
            warn!("failed to send a plain message - {err}")
        }
    }

    async fn handle_repliable_message(
        &mut self,
        recipient: Recipient,
        content: Vec<u8>,
        reply_surbs: u32,
        lane: TransmissionLane,
    ) {
        if let Err(err) = self
            .message_handler
            .try_send_message_with_reply_surbs(recipient, content, reply_surbs, lane)
            .await
        {
            warn!("failed to send a repliable message - {err}")
        }
    }

    async fn on_input_message(&mut self, msg: InputMessage) {
        match msg {
            InputMessage::Regular {
                recipient,
                data,
                lane,
            } => self.handle_plain_message(recipient, data, lane).await,
            InputMessage::Anonymous {
                recipient,
                data,
                reply_surbs,
                lane,
            } => {
                self.handle_repliable_message(recipient, data, reply_surbs, lane)
                    .await
            }
            InputMessage::Reply {
                recipient_tag,
                data,
                lane,
            } => {
                self.handle_reply(recipient_tag, data, lane).await;
            }
        };
    }

    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        debug!("Started InputMessageListener with graceful shutdown support");

        while !shutdown.is_shutdown() {
            tokio::select! {
                input_msg = self.input_receiver.recv() => match input_msg {
                    Some(input_msg) => {
                        self.on_input_message(input_msg).await;
                    },
                    None => {
                        log::trace!("InputMessageListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = shutdown.recv_with_delay() => {
                    log::trace!("InputMessageListener: Received shutdown");
                }
            }
        }
        shutdown.recv_timeout().await;
        log::debug!("InputMessageListener: Exiting");
    }
}
