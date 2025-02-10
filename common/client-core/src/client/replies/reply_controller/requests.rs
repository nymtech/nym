// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use futures::channel::{mpsc, oneshot};
use log::error;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::anonymous_replies::ReplySurb;
use nym_task::connections::{ConnectionId, TransmissionLane};
use std::sync::Weak;

pub(crate) fn new_control_channels() -> (ReplyControllerSender, ReplyControllerReceiver) {
    let (tx, rx) = mpsc::unbounded();
    (tx.into(), rx)
}

#[derive(Debug, Clone)]
pub struct ReplyControllerSender(mpsc::UnboundedSender<ReplyControllerMessage>);

impl From<mpsc::UnboundedSender<ReplyControllerMessage>> for ReplyControllerSender {
    fn from(inner: mpsc::UnboundedSender<ReplyControllerMessage>) -> Self {
        ReplyControllerSender(inner)
    }
}

impl ReplyControllerSender {
    pub(crate) fn send_retransmission_data(
        &self,
        recipient: AnonymousSenderTag,
        timed_out_ack: Weak<PendingAcknowledgement>,
        extra_surb_request: bool,
    ) {
        if let Err(err) = self
            .0
            .unbounded_send(ReplyControllerMessage::RetransmitReply {
                recipient,
                timed_out_ack,
                extra_surb_request,
            })
        {
            error!("Failed to send retransmission data to reply controller: {err}",);
        }
    }

    pub(crate) fn send_reply(
        &self,
        recipient: AnonymousSenderTag,
        message: Vec<u8>,
        lane: TransmissionLane,
    ) {
        if let Err(err) = self.0.unbounded_send(ReplyControllerMessage::SendReply {
            recipient,
            message,
            lane,
        }) {
            error!("Failed to send reply to reply controller: {err}",);
        }
    }

    pub(crate) fn send_additional_surbs(
        &self,
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    ) {
        if let Err(err) = self
            .0
            .unbounded_send(ReplyControllerMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
                from_surb_request,
            })
        {
            error!("Failed to send additional surbs to reply controller: {err}",);
        }
    }

    pub(crate) fn send_additional_surbs_request(&self, recipient: Recipient, amount: u32) {
        if let Err(err) = self
            .0
            .unbounded_send(ReplyControllerMessage::AdditionalSurbsRequest {
                recipient: Box::new(recipient),
                amount,
            })
        {
            error!("Failed to send additional surbs request to reply controller: {err}");
        }
    }

    pub async fn get_lane_queue_length(&self, connection_id: ConnectionId) -> usize {
        let (response_tx, response_rx) = oneshot::channel();
        if let Err(err) = self
            .0
            .unbounded_send(ReplyControllerMessage::LaneQueueLength {
                connection_id,
                response_channel: response_tx,
            })
        {
            error!("Failed to send lane queue length request to reply controller: {err}");
        }

        match response_rx.await {
            Ok(length) => length,
            Err(_) => {
                error!("The reply controller has dropped our response channel!");
                // TODO: should we panic here instead? this message implies something weird and unrecoverable has happened
                0
            }
        }
    }
}

pub struct ReplyQueueLengths {
    reply_controller_sender: ReplyControllerSender,
}

impl ReplyQueueLengths {
    pub fn new(reply_controller_sender: ReplyControllerSender) -> Self {
        Self {
            reply_controller_sender,
        }
    }

    pub async fn get_lane_queue_length(&self, connection_id: ConnectionId) -> usize {
        self.reply_controller_sender
            .get_lane_queue_length(connection_id)
            .await
    }
}

pub(crate) type ReplyControllerReceiver = mpsc::UnboundedReceiver<ReplyControllerMessage>;

#[derive(Debug)]
pub(crate) enum ReplyControllerMessage {
    RetransmitReply {
        recipient: AnonymousSenderTag,
        timed_out_ack: Weak<PendingAcknowledgement>,
        extra_surb_request: bool,
    },

    SendReply {
        recipient: AnonymousSenderTag,
        message: Vec<u8>,
        lane: TransmissionLane,
    },

    AdditionalSurbs {
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    },

    // this one doesn't belong here either...
    LaneQueueLength {
        connection_id: ConnectionId,
        response_channel: oneshot::Sender<usize>,
    },

    // Should this also be handled in here? it's technically a completely different side of the pipe
    // let's see how it works when combined, might split it before creating PR
    AdditionalSurbsRequest {
        recipient: Box<Recipient>,
        amount: u32,
    },
}
