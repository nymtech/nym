//! Simple reliable broadcast protocol(Bracha) implementation
//!
use std::collections::HashSet;
use std::fmt::{Debug, Display};

use serde_derive::{Deserialize, Serialize};

use crate::broadcast::bracha::quorum::Quorum;
use crate::{
    block::types::block::Block,
    peer::PeerId,
    utilities::{
        crypto::Certificate,
        hash::Hash,
        id::{EphemeraId, EphemeraIdentifier},
        time::EphemeraTime,
    },
};

pub(crate) mod bracha;
pub(crate) mod group;
pub(crate) mod signing;

/// Context keeps the broadcast state for a block
#[derive(Debug, Clone)]
pub(crate) struct ProtocolContext {
    pub(crate) local_peer_id: PeerId,
    /// Block hash
    pub(crate) hash: Hash,
    /// Peers that sent prepare message(this peer included)
    pub(crate) echo: HashSet<PeerId>,
    /// Peers that sent commit message(this peer included)
    pub(crate) vote: HashSet<PeerId>,
    /// Quorum logic for Bracha protocol
    pub(crate) quorum: Quorum,
    /// Flag indicating if the message was delivered to the client
    pub(crate) delivered: bool,
}

impl ProtocolContext {
    pub(crate) fn new(hash: Hash, local_peer_id: PeerId, quorum: Quorum) -> ProtocolContext {
        ProtocolContext {
            local_peer_id,
            hash,
            echo: HashSet::new(),
            vote: HashSet::new(),
            quorum,
            delivered: false,
        }
    }

    fn add_echo(&mut self, peer: PeerId) {
        self.echo.insert(peer);
    }

    fn add_vote(&mut self, peer: PeerId) {
        self.vote.insert(peer);
    }

    fn echoed(&self) -> bool {
        self.echo.contains(&self.local_peer_id)
    }

    fn voted(&self) -> bool {
        self.vote.contains(&self.local_peer_id)
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct RbMsg {
    ///Unique id of the message which stays the same throughout the protocol
    pub(crate) id: EphemeraId,
    ///Distinct id of the message which changes when the message is rebroadcast
    pub(crate) request_id: EphemeraId,
    ///Id of the peer that CREATED the message(not necessarily the one that sent it, with gossip it can come through a different peer)
    pub(crate) original_sender: PeerId,
    ///When the message was created by the sender.
    pub(crate) timestamp: u64,
    ///Current phase of the protocol(Echo, Vote)
    pub(crate) phase: MessageType,
    ///Signature of the message
    pub(crate) certificate: Certificate,
}

impl RbMsg {
    pub(crate) fn new(raw: RawRbMsg, signature: Certificate) -> RbMsg {
        RbMsg {
            id: raw.id,
            request_id: raw.request_id,
            original_sender: raw.original_sender,
            timestamp: raw.timestamp,
            phase: raw.message_type,
            certificate: signature,
        }
    }

    pub(crate) fn block(&self) -> &Block {
        match &self.phase {
            MessageType::Echo(block) | MessageType::Vote(block) => block,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct RawRbMsg {
    pub(crate) id: EphemeraId,
    pub(crate) request_id: EphemeraId,
    pub(crate) original_sender: PeerId,
    pub(crate) timestamp: u64,
    pub(crate) message_type: MessageType,
}

impl RawRbMsg {
    pub(crate) fn new(block: Block, original_sender: PeerId) -> RawRbMsg {
        RawRbMsg {
            id: EphemeraId::generate(),
            request_id: EphemeraId::generate(),
            original_sender,
            timestamp: EphemeraTime::now(),
            message_type: MessageType::Echo(block),
        }
    }

    pub(crate) fn block(&self) -> Block {
        match &self.message_type {
            MessageType::Echo(block) | MessageType::Vote(block) => block.clone(),
        }
    }

    pub(crate) fn reply(&self, local_id: PeerId, phase: MessageType) -> Self {
        RawRbMsg {
            id: self.id.clone(),
            request_id: EphemeraId::generate(),
            original_sender: local_id,
            timestamp: EphemeraTime::now(),
            message_type: phase,
        }
    }

    pub(crate) fn echo_reply(&self, local_id: PeerId, data: Block) -> Self {
        self.reply(local_id, MessageType::Echo(data))
    }

    pub(crate) fn vote_reply(&self, local_id: PeerId, data: Block) -> Self {
        self.reply(local_id, MessageType::Vote(data))
    }
}

impl From<RbMsg> for RawRbMsg {
    fn from(msg: RbMsg) -> Self {
        RawRbMsg {
            id: msg.id,
            request_id: msg.request_id,
            original_sender: msg.original_sender,
            timestamp: msg.timestamp,
            message_type: msg.phase,
        }
    }
}

impl Display for RbMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[id: {}, peer: {}, block: {}, phase: {:?}]",
            self.id,
            self.original_sender,
            self.block().get_hash(),
            self.phase
        )
    }
}

impl Debug for RbMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[id: {}, peer: {}, block: {}, phase: {:?}]",
            self.id,
            self.original_sender,
            self.block().get_hash(),
            self.phase
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum MessageType {
    Echo(Block),
    Vote(Block),
}
