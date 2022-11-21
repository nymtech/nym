// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::inbound_messages::{InputMessage, InputMessageReceiver};
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::temp_name_pending_handler::ToBeNamedSender;
use futures::StreamExt;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::{AnonymousSenderTag, ReplyMessage};
use nymsphinx::anonymous_replies::ReplySurb;
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
    to_be_named_channel: ToBeNamedSender,
}

pub(super) struct Config {
    max_per_sender_buffer_size: usize,
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
        to_be_named_channel: ToBeNamedSender,
    ) -> Self {
        InputMessageListener {
            input_receiver,
            message_handler,
            to_be_named_channel,
        }
    }

    async fn handle_reply(&mut self, recipient_tag: AnonymousSenderTag, data: Vec<u8>) {
        // offload reply handling to the dedicated task
        self.to_be_named_channel.send_reply(recipient_tag, data)
    }

    // we require topology for replies to generate surb_acks
    async fn handle_reply_with_surb(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        reply_surb: ReplySurb,
        data: Vec<u8>,
    ) {
        let message = ReplyMessage::new_data_message(data);
        if let Err(_returned_surb) = self
            .message_handler
            .try_send_single_surb_message(recipient_tag, message, reply_surb, false)
            .await
        {
            // TODO: return concrete error instead
            warn!("failed to send our single-surb message. It was either too long or the topology was invalid");
        }
    }

    async fn handle_plain_message(&mut self, recipient: Recipient, content: Vec<u8>) {
        if let Err(err) = self
            .message_handler
            .try_send_plain_message(recipient, content)
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
    ) {
        if let Err(err) = self
            .message_handler
            .try_send_message_with_reply_surbs(recipient, content, reply_surbs)
            .await
        {
            warn!("failed to send a repliable message - {err}")
        }
    }

    async fn on_input_message(&mut self, msg: InputMessage) {
        match msg {
            InputMessage::Regular { recipient, data } => {
                self.handle_plain_message(recipient, data).await
            }
            InputMessage::Anonymous {
                recipient,
                data,
                reply_surbs,
            } => {
                self.handle_repliable_message(recipient, data, reply_surbs)
                    .await
            }
            InputMessage::Reply {
                recipient_tag,
                data,
            } => {
                self.handle_reply(recipient_tag, data).await;
            }
            InputMessage::ReplyWithSurb {
                recipient_tag,
                reply_surb,
                data,
            } => {
                self.handle_reply_with_surb(recipient_tag, reply_surb, data)
                    .await
            }
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started InputMessageListener with graceful shutdown support");

        while !shutdown.is_shutdown() {
            tokio::select! {
                input_msg = self.input_receiver.next() => match input_msg {
                    Some(input_msg) => {
                        self.on_input_message(input_msg).await;
                    },
                    None => {
                        log::trace!("InputMessageListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = shutdown.recv() => {
                    log::trace!("InputMessageListener: Received shutdown");
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("InputMessageListener: Exiting");
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn run(&mut self) {
        debug!("Started InputMessageListener without graceful shutdown support");
        while let Some(input_msg) = self.input_receiver.next().await {
            self.on_input_message(input_msg).await;
        }
    }
}
