// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What a client hands the LP path to send.

use crate::client::lp::data::handler::pipeline::outbound::LpOutboundOptions;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use tokio::sync::mpsc;

/// The largest message the LP path takes in one piece, in bytes.
///
/// LP neither acknowledges nor retransmits, so a message arrives only if every one of its packets
/// does, and the odds of that fall with every packet it takes. Bigger data belongs on a stream.
pub const MAX_LP_MESSAGE_SIZE: usize = 64 * 1024;

pub(crate) type LpOutboundMessageSender = mpsc::Sender<LpOutboundMessage>;
pub(crate) type LpOutboundMessageReceiver = mpsc::Receiver<LpOutboundMessage>;

/// A message to send over LP, and everything about how it is sent.
///
/// Built with what the caller cares about; anything left out takes its default, which for the
/// gateway is the one this client registered with.
#[derive(Debug)]
pub struct LpOutboundMessage {
    pub(crate) data: Vec<u8>,

    pub(crate) options: LpOutboundOptions,

    /// The gateway the message leaves through; `None` for this client's main gateway.
    pub(crate) gateway: Option<NodeIdentity>,
}

impl LpOutboundMessage {
    /// Send `data` with these options, through this client's main gateway.
    pub fn new(data: Vec<u8>, options: LpOutboundOptions) -> Self {
        LpOutboundMessage {
            data,
            options,
            gateway: None,
        }
    }

    /// Send `data` to `recipient`, with every other option at its default.
    pub fn to_recipient(recipient: Recipient, data: Vec<u8>) -> Self {
        Self::new(data, LpOutboundOptions { recipient })
    }

    /// Leave through `gateway` rather than this client's main one.
    ///
    /// The client has to hold an LP session with it; a message for a gateway it has none with is
    /// dropped.
    #[must_use]
    pub fn via_gateway(mut self, gateway: NodeIdentity) -> Self {
        self.gateway = Some(gateway);
        self
    }
}
