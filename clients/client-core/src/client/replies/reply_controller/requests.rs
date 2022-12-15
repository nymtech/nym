// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use client_connections::TransmissionLane;
use futures::channel::mpsc;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::ReplySurb;
use std::sync::Weak;

pub(crate) fn new_control_channels() -> (ReplyControllerSender, ReplyControllerReceiver) {
    let (tx, rx) = mpsc::unbounded();
    (tx.into(), rx)
}

#[derive(Debug, Clone)]
pub(crate) struct ReplyControllerSender(mpsc::UnboundedSender<ReplyControllerMessage>);

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
        self.0
            .unbounded_send(ReplyControllerMessage::RetransmitReply {
                recipient,
                timed_out_ack,
                extra_surb_request,
            })
            .expect("ReplyControllerReceiver has died!")
    }

    pub(crate) fn send_reply(
        &self,
        recipient: AnonymousSenderTag,
        message: Vec<u8>,
        lane: TransmissionLane,
    ) {
        self.0
            .unbounded_send(ReplyControllerMessage::SendReply {
                recipient,
                message,
                lane,
            })
            .expect("ReplyControllerReceiver has died!")
    }

    pub(crate) fn send_additional_surbs(
        &self,
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    ) {
        self.0
            .unbounded_send(ReplyControllerMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
                from_surb_request,
            })
            .expect("ReplyControllerReceiver has died!")
    }

    pub(crate) fn send_additional_surbs_request(&self, recipient: Recipient, amount: u32) {
        self.0
            .unbounded_send(ReplyControllerMessage::AdditionalSurbsRequest {
                recipient: Box::new(recipient),
                amount,
            })
            .expect("ReplyControllerReceiver has died!")
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

    // Should this also be handled in here? it's technically a completely different side of the pipe
    // let's see how it works when combined, might split it before creating PR
    AdditionalSurbsRequest {
        recipient: Box<Recipient>,
        amount: u32,
    },
}
