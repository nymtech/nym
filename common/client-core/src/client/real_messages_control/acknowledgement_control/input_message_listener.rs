// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::inbound_messages::{InputMessage, InputMessageReceiver};
use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::real_messages_control::real_traffic_stream::RealMessage;
use crate::client::replies::reply_controller::ReplyControllerSender;
use log::*;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;

/// Module responsible for dealing with the received messages: splitting them, creating acknowledgements,
/// putting everything into sphinx packets, etc.
/// It also makes an initial sending attempt for said messages.
pub(super) struct InputMessageListener {
    input_receiver: InputMessageReceiver,
    message_handler: MessageHandler,
    reply_controller_sender: ReplyControllerSender,
}

impl InputMessageListener {
    // at this point I'm not entirely sure how to deal with this warning without
    // some considerable refactoring
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        input_receiver: InputMessageReceiver,
        message_handler: MessageHandler,
        reply_controller_sender: ReplyControllerSender,
    ) -> Self {
        InputMessageListener {
            input_receiver,
            message_handler,
            reply_controller_sender,
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
        packet_type: PacketType,
        mix_hops: Option<u8>,
    ) {
        if let Err(err) = self
            .message_handler
            .try_send_plain_message(recipient, content, lane, packet_type, mix_hops)
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
        mix_hops: Option<u8>,
    ) {
        if let Err(err) = self
            .message_handler
            .try_send_message_with_reply_surbs(
                recipient,
                content,
                reply_surbs,
                lane,
                packet_type,
                mix_hops,
            )
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
                mix_hops,
            } => {
                self.handle_plain_message(recipient, data, lane, PacketType::Mix, mix_hops)
                    .await
            }
            InputMessage::Anonymous {
                recipient,
                data,
                reply_surbs,
                lane,
                mix_hops,
            } => {
                self.handle_repliable_message(
                    recipient,
                    data,
                    reply_surbs,
                    lane,
                    PacketType::Mix,
                    mix_hops,
                )
                .await
            }
            InputMessage::Reply {
                recipient_tag,
                data,
                lane,
            } => {
                self.handle_reply(recipient_tag, data, lane).await;
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
                    mix_hops,
                } => {
                    self.handle_plain_message(recipient, data, lane, packet_type, mix_hops)
                        .await
                }
                InputMessage::Anonymous {
                    recipient,
                    data,
                    reply_surbs,
                    lane,
                    mix_hops,
                } => {
                    self.handle_repliable_message(
                        recipient,
                        data,
                        reply_surbs,
                        lane,
                        packet_type,
                        mix_hops,
                    )
                    .await
                }
                InputMessage::Reply {
                    recipient_tag,
                    data,
                    lane,
                } => {
                    self.handle_reply(recipient_tag, data, lane).await;
                }
                InputMessage::Premade { msgs, lane } => {
                    self.handle_premade_packets(msgs, lane).await
                }
                // MessageWrappers can't be nested
                InputMessage::MessageWrapper { .. } => unimplemented!(),
            },
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
