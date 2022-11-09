// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::real_traffic_stream::{
    BatchRealMessageSender, RealMessage,
};
use crate::client::replies::reply_storage::CombinedReplyStorage;
use log::debug;
use nymsphinx::acknowledgements::surb_ack::SurbAck;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::params::PacketSize;
use std::collections::HashMap;

pub(crate) enum ReplyRequest {
    SendReply {
        recipient_tag: AnonymousSenderTag,
    },
    SendReplySurbs {
        //
    },
}

// TODO: move elsewhere
struct PendingReply {
    packet_payload: Vec<u8>,
    surb_ack: SurbAck,
}

pub(crate) struct PendingReplyHandler {
    expected_reliability: f32,
    packet_size_used: PacketSize,
    reply_storage: CombinedReplyStorage,

    pending_replies: HashMap<AnonymousSenderTag, PendingReply>,
    real_message_sender: BatchRealMessageSender,
}

impl PendingReplyHandler {
    fn request_more_surbs(&self) {
        //
    }

    fn handle_send_reply(&self) {
        //

        // don't even attempt sending the message if we know we don't have enough surbs
        // to send it through.
        // request the surbs and then try again
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started AcknowledgementListener with graceful shutdown support");

        while !shutdown.is_shutdown() {
            tokio::select! {
                // acks = self.ack_receiver.next() => match acks {
                //     Some(acks) => self.handle_ack_receiver_item(acks).await,
                //     None => {
                //         log::trace!("AcknowledgementListener: Stopping since channel closed");
                //         break;
                //     }
                // },
                _ = shutdown.recv() => {
                    log::trace!("AcknowledgementListener: Received shutdown");
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("AcknowledgementListener: Exiting");
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn run(&mut self) {
        debug!("Started AcknowledgementListener without graceful shutdown support");

        while let Some(acks) = self.ack_receiver.next().await {
            self.handle_ack_receiver_item(acks).await
        }
    }
}
