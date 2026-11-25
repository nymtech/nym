// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sdk::mixnet::{AnonymousSenderTag, MixnetClientSender, MixnetMessageSender};
use nym_service_providers_common::lp::handler::outbound::{ReplyTarget, ServiceProviderReply};

use crate::error::{IpPacketRouterError, Result};
use crate::util::create_message::create_input_message;

/// Where replies to a client go: back the way that client reached this router.
#[derive(Clone)]
pub(crate) enum ReplySender {
    /// Through the mixnet client.
    Mixnet(MixnetClientSender),

    /// Over LP, where the pipeline answers through the sender's SURBs.
    Lp(tokio::sync::mpsc::Sender<ServiceProviderReply>),
}

impl ReplySender {
    /// `max_retransmissions` only applies to the mixnet path; nothing on LP is retransmitted.
    pub(crate) async fn send(
        &self,
        to: &AnonymousSenderTag,
        data: Vec<u8>,
        max_retransmissions: Option<u32>,
    ) -> Result<()> {
        match self {
            ReplySender::Mixnet(sender) => {
                let mut message = create_input_message(to, data);
                if let Some(max_retransmissions) = max_retransmissions {
                    message = message.with_max_retransmissions(max_retransmissions);
                }

                sender.send(message).await.map_err(|source| {
                    IpPacketRouterError::FailedToSendPacketToMixnet {
                        source: Box::new(source),
                    }
                })
            }
            ReplySender::Lp(sender) => sender
                .send(ServiceProviderReply {
                    data,
                    to: ReplyTarget::SenderTag(*to),
                })
                .await
                .map_err(|_| IpPacketRouterError::LpDataPlaneClosed),
        }
    }
}
