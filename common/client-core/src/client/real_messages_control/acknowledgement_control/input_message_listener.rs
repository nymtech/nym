// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::inbound_messages::{InputMessage, InputMessageReceiver};
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::real_messages_control::real_traffic_stream::RealMessage;
use crate::client::replies::reply_controller::ReplyControllerSender;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;
use nym_task::TaskClient;
use rand::{CryptoRng, Rng};
use tracing::*;

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
    task_client: TaskClient,
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
        task_client: TaskClient,
    ) -> Self {
        InputMessageListener {
            input_receiver,
            message_handler,
            reply_controller_sender,
            task_client,
        }
    }

    async fn handle_premade_packets(&mut self, packets: Vec<MixPacket>, lane: TransmissionLane) {
        self.message_handler
            .send_premade_mix_packets(
                packets
                    .into_iter()
                    .map(|p| RealMessage::new(p, None))
                    .collect(),
                lane,
            )
            .await
    }

    async fn handle_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
        max_retransmissions: Option<u32>,
    ) {
        // offload reply handling to the dedicated task
        if let Err(err) =
            self.reply_controller_sender
                .send_reply(recipient_tag, data, lane, max_retransmissions)
        {
            if !self.task_client.is_shutdown_poll() {
                error!("failed to send a reply - {err}");
            }
        }
    }

    async fn handle_plain_message(
        &mut self,
        recipient: Recipient,
        content: Vec<u8>,
        lane: TransmissionLane,
        packet_type: PacketType,
        max_retransmissions: Option<u32>,
    ) {
        if let Err(err) = self
            .message_handler
            .try_send_plain_message(recipient, content, lane, packet_type, max_retransmissions)
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
        packet_type: PacketType,
        max_retransmissions: Option<u32>,
    ) {
        if let Err(err) = self
            .message_handler
            .try_send_message_with_reply_surbs(
                recipient,
                content,
                reply_surbs,
                lane,
                packet_type,
                max_retransmissions,
            )
            .await
        {
            warn!("failed to send a repliable message - {err}")
        }
    }

    #[allow(clippy::panic)]
    async fn on_input_message(&mut self, msg: InputMessage) {
        match msg {
            InputMessage::Regular {
                recipient,
                data,
                lane,
                max_retransmissions,
            } => {
                self.handle_plain_message(
                    recipient,
                    data,
                    lane,
                    PacketType::Mix,
                    max_retransmissions,
                )
                .await
            }
            InputMessage::Anonymous {
                recipient,
                data,
                reply_surbs,
                lane,
                max_retransmissions,
            } => {
                self.handle_repliable_message(
                    recipient,
                    data,
                    reply_surbs,
                    lane,
                    PacketType::Mix,
                    max_retransmissions,
                )
                .await
            }
            InputMessage::Reply {
                recipient_tag,
                data,
                lane,
                max_retransmissions,
            } => {
                self.handle_reply(recipient_tag, data, lane, max_retransmissions)
                    .await;
            }
            InputMessage::Premade { msgs, lane } => self.handle_premade_packets(msgs, lane).await,
            InputMessage::MessageWrapper {
                message,
                packet_type,
            } => match *message {
                InputMessage::Regular {
                    recipient,
                    data,
                    lane,
                    max_retransmissions,
                } => {
                    self.handle_plain_message(
                        recipient,
                        data,
                        lane,
                        packet_type,
                        max_retransmissions,
                    )
                    .await
                }
                InputMessage::Anonymous {
                    recipient,
                    data,
                    reply_surbs,
                    lane,
                    max_retransmissions,
                } => {
                    self.handle_repliable_message(
                        recipient,
                        data,
                        reply_surbs,
                        lane,
                        packet_type,
                        max_retransmissions,
                    )
                    .await
                }
                InputMessage::Reply {
                    recipient_tag,
                    data,
                    lane,
                    max_retransmissions,
                } => {
                    self.handle_reply(recipient_tag, data, lane, max_retransmissions)
                        .await;
                }
                InputMessage::Premade { msgs, lane } => {
                    self.handle_premade_packets(msgs, lane).await
                }
                // MessageWrappers can't be nested
                InputMessage::MessageWrapper { .. } => {
                    panic!("attempted to use nested MessageWrapper")
                }
            },
        };
    }

    pub(super) async fn run(&mut self) {
        debug!("Started InputMessageListener with graceful shutdown support");

        while !self.task_client.is_shutdown() {
            tokio::select! {
                input_msg = self.input_receiver.recv() => match input_msg {
                    Some(input_msg) => {
                        self.on_input_message(input_msg).await;
                    },
                    None => {
                        tracing::trace!("InputMessageListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = self.task_client.recv() => {
                    tracing::trace!("InputMessageListener: Received shutdown");
                }
            }
        }
        self.task_client.recv_timeout().await;
        tracing::debug!("InputMessageListener: Exiting");
    }
}
