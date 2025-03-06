// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt,
};
use log::error;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::anonymous_replies::ReplySurb;
use nym_task::connections::{ConnectionId, TransmissionLane};
use std::sync::Weak;

pub(crate) fn new_control_channels() -> (ReplyControllerSender, ReplyControllerReceiver) {
    let (tx, rx) = mpsc::channel(8);
    (tx.into(), rx)
}

#[derive(Debug, thiserror::Error)]
pub enum ReplyControllerSenderError {
    #[error("failed to send retransmission data to reply controller")]
    // SendRetransmissionData(#[source] mpsc::TrySendError<ReplyControllerMessage>),
    SendRetransmissionData,

    #[error("failed to send reply to reply controller")]
    // SendReply(#[source] mpsc::TrySendError<ReplyControllerMessage>),
    SendReply,

    #[error("failed to send additional surbs to reply controller")]
    // AdditionalSurbs(#[source] mpsc::TrySendError<ReplyControllerMessage>),
    AdditionalSurbs,

    #[error("failed to send additional surbs request to reply controller")]
    // AdditionalSurbsRequest(#[source] mpsc::TrySendError<ReplyControllerMessage>),
    AdditionalSurbsRequest,

    #[error("failed to request lane queue length from reply controller")]
    // LaneQueueLength(#[source] mpsc::TrySendError<ReplyControllerMessage>),
    LaneQueueLength,

    #[error("response channel was dropped before we could receive the response")]
    // ResponseChannelDropped(#[source] oneshot::Canceled),
    ResponseChannelDropped,
}

#[derive(Debug, Clone)]
pub struct ReplyControllerSender(mpsc::Sender<ReplyControllerMessage>);

impl From<mpsc::Sender<ReplyControllerMessage>> for ReplyControllerSender {
    fn from(inner: mpsc::Sender<ReplyControllerMessage>) -> Self {
        ReplyControllerSender(inner)
    }
}

impl ReplyControllerSender {
    pub(crate) async fn send_retransmission_data(
        &mut self,
        recipient: AnonymousSenderTag,
        timed_out_ack: Weak<PendingAcknowledgement>,
        extra_surb_request: bool,
    ) -> Result<(), ReplyControllerSenderError> {
        self.0
            .send(ReplyControllerMessage::RetransmitReply {
                recipient,
                timed_out_ack,
                extra_surb_request,
            })
            .await
            .map_err(|_| ReplyControllerSenderError::SendRetransmissionData)
    }

    pub(crate) async fn send_reply(
        &mut self,
        recipient: AnonymousSenderTag,
        message: Vec<u8>,
        lane: TransmissionLane,
    ) -> Result<(), ReplyControllerSenderError> {
        self.0
            .send(ReplyControllerMessage::SendReply {
                recipient,
                message,
                lane,
            })
            .await
            .map_err(|_| ReplyControllerSenderError::SendReply)
    }

    pub(crate) async fn send_additional_surbs(
        &mut self,
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    ) -> Result<(), ReplyControllerSenderError> {
        self.0
            .send(ReplyControllerMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
                from_surb_request,
            })
            .await
            .map_err(|_| ReplyControllerSenderError::AdditionalSurbs)
    }

    pub(crate) async fn send_additional_surbs_request(
        &mut self,
        recipient: Recipient,
        amount: u32,
    ) -> Result<(), ReplyControllerSenderError> {
        self.0
            .send(ReplyControllerMessage::AdditionalSurbsRequest {
                recipient: Box::new(recipient),
                amount,
            })
            .await
            .map_err(|_| ReplyControllerSenderError::AdditionalSurbsRequest)
    }

    pub async fn get_lane_queue_length(
        &mut self,
        connection_id: ConnectionId,
    ) -> Result<usize, ReplyControllerSenderError> {
        let (response_tx, response_rx) = oneshot::channel();
        if let Err(_err) = self
            .0
            .send(ReplyControllerMessage::LaneQueueLength {
                connection_id,
                response_channel: response_tx,
            })
            .await
        {
            return Err(ReplyControllerSenderError::LaneQueueLength);
        }

        response_rx
            .await
            .map_err(|_| ReplyControllerSenderError::ResponseChannelDropped)
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

    pub async fn get_lane_queue_length(
        &mut self,
        connection_id: ConnectionId,
    ) -> Result<usize, ReplyControllerSenderError> {
        self.reply_controller_sender
            .get_lane_queue_length(connection_id)
            .await
    }
}

pub(crate) type ReplyControllerReceiver = mpsc::Receiver<ReplyControllerMessage>;

#[derive(Debug)]
pub enum ReplyControllerMessage {
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
