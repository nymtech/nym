use std::num::NonZeroUsize;

use log::{debug, trace};
use lru::LruCache;

use crate::broadcast::bracha::quorum::BrachaMessageType;
use crate::peer::PeerId;
use crate::{
    block::types::block::Block,
    broadcast::{
        bracha::quorum::Quorum,
        MessageType::{Echo, Vote},
        ProtocolContext, RawRbMsg,
    },
    utilities::hash::Hash,
};

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum BroadcastResponse {
    Broadcast(RawRbMsg),
    Deliver(Hash),
    Drop(Hash),
}

pub(crate) struct Broadcaster {
    /// Local peer id
    local_peer_id: PeerId,
    /// We keep a context for each block we are processing.
    contexts: LruCache<Hash, ProtocolContext>,
    /// Current cluster size
    cluster_size: usize,
}

impl Broadcaster {
    pub fn new(peer_id: PeerId) -> Broadcaster {
        Broadcaster {
            //At any given time we are processing in parallel about n messages, where n is the number of peers in the group.
            //This is just large enough buffer.
            contexts: LruCache::new(NonZeroUsize::new(1000).unwrap()),
            cluster_size: 0,
            local_peer_id: peer_id,
        }
    }

    pub(crate) fn new_broadcast(&mut self, block: Block) -> anyhow::Result<BroadcastResponse> {
        debug!("Starting broadcast for new block {:?}", block.get_hash());
        self.handle(&RawRbMsg::new(block, self.local_peer_id))
    }

    pub(crate) fn handle(&mut self, rb_msg: &RawRbMsg) -> anyhow::Result<BroadcastResponse> {
        trace!("Processing new broadcast message: {:?}", rb_msg);

        let block = rb_msg.block();
        let hash = block.hash_with_default_hasher()?;

        let ctx = self.contexts.get_or_insert(hash, || {
            ProtocolContext::new(hash, self.local_peer_id, Quorum::new(self.cluster_size))
        });

        if ctx.delivered {
            trace!("Block {hash:?} already delivered");
            return Ok(BroadcastResponse::Drop(hash));
        }

        match rb_msg.message_type {
            Echo(_) => {
                trace!("Processing ECHO {:?}", rb_msg.id);
                Ok(self.process_echo(rb_msg, hash))
            }
            Vote(_) => {
                trace!("Processing VOTE {:?}", rb_msg.id);
                Ok(self.process_vote(rb_msg, hash))
            }
        }
    }

    fn process_echo(&mut self, rb_msg: &RawRbMsg, hash: Hash) -> BroadcastResponse {
        let ctx = self.contexts.get_mut(&hash).expect("Context not found");

        if self.local_peer_id != rb_msg.original_sender {
            trace!("Adding echo from {:?}", rb_msg.original_sender);
            ctx.add_echo(rb_msg.original_sender);
        }

        if !ctx.echoed() {
            ctx.add_echo(self.local_peer_id);

            trace!("Sending echo reply for {hash:?}",);
            return BroadcastResponse::Broadcast(
                rb_msg.echo_reply(self.local_peer_id, rb_msg.block()),
            );
        }

        if !ctx.voted()
            && ctx
                .quorum
                .check_threshold(ctx, BrachaMessageType::Echo)
                .is_vote()
        {
            ctx.add_vote(self.local_peer_id);

            trace!("Sending vote reply for {hash:?}",);
            return BroadcastResponse::Broadcast(
                rb_msg.vote_reply(self.local_peer_id, rb_msg.block()),
            );
        }

        BroadcastResponse::Drop(hash)
    }

    fn process_vote(&mut self, rb_msg: &RawRbMsg, hash: Hash) -> BroadcastResponse {
        let block = rb_msg.block();
        let ctx = self.contexts.get_mut(&hash).expect("Context not found");

        if self.local_peer_id != rb_msg.original_sender {
            trace!("Adding vote from {:?}", rb_msg.original_sender);
            ctx.add_vote(rb_msg.original_sender);
        }

        if ctx
            .quorum
            .check_threshold(ctx, BrachaMessageType::Vote)
            .is_vote()
        {
            ctx.add_vote(self.local_peer_id);

            trace!("Sending vote reply for {hash:?}",);
            return BroadcastResponse::Broadcast(rb_msg.vote_reply(self.local_peer_id, block));
        }

        if ctx
            .quorum
            .check_threshold(ctx, BrachaMessageType::Vote)
            .is_deliver()
        {
            trace!("Commit complete for {:?}", rb_msg.id);

            ctx.delivered = true;

            return BroadcastResponse::Deliver(hash);
        }

        BroadcastResponse::Drop(hash)
    }

    pub(crate) fn group_updated(&mut self, size: usize) {
        self.cluster_size = size;
    }
}

#[cfg(test)]
mod tests {

    //1.make sure before voting enough echo messages are received
    //2.make sure before delivering enough vote messages are received
    //a)Either f + 1
    //b)Or n - f

    //3.make sure that duplicate messages doesn't have impact

    //4. "Ideally" make sure that when group changes, the ongoing broadcast can deal with it

    use std::iter;

    use assert_matches::assert_matches;

    use crate::broadcast::bracha::broadcast::BroadcastResponse;
    use crate::peer::PeerId;
    use crate::utilities::hash::Hash;
    use crate::{
        block::types::block::{Block, RawBlock, RawBlockHeader},
        broadcast::{self, bracha::broadcast::Broadcaster, RawRbMsg},
    };

    #[test]
    fn test_state_transitions_from_start_to_end() {
        let peers: Vec<PeerId> = iter::repeat_with(PeerId::random).take(10).collect();
        let local_peer_id = peers[0];
        let block_creator_peer_id = peers[1];

        let mut broadcaster = Broadcaster::new(local_peer_id);
        broadcaster.group_updated(peers.len());

        let (block_hash, block) = create_block(block_creator_peer_id);

        //After this echo set contains local and block creator(msg sender)
        receive_echo_first_message(&mut broadcaster, &block, block_creator_peer_id);

        let ctx = broadcaster.contexts.get(&block_hash).unwrap();
        assert_eq!(ctx.echo.len(), 2);
        assert!(ctx.echoed());
        assert!(!ctx.voted());

        receive_nr_of_echo_messages_below_vote_threshold(&mut broadcaster, &block, &peers[2..6]);

        let ctx = broadcaster.contexts.get(&block_hash).unwrap();
        assert_eq!(ctx.echo.len(), 6);
        assert!(ctx.echoed());
        assert!(!ctx.voted());

        receive_echo_threshold_message(&mut broadcaster, &block, *peers.get(7).unwrap());

        let ctx = broadcaster.contexts.get(&block_hash).unwrap();
        assert_eq!(ctx.echo.len(), 7);
        assert_eq!(ctx.vote.len(), 1);
        assert!(ctx.echoed());
        assert!(ctx.voted());

        receive_nr_of_vote_messages_below_deliver_threshold(&mut broadcaster, &block, &peers[2..7]);

        let ctx = broadcaster.contexts.get(&block_hash).unwrap();
        assert_eq!(ctx.echo.len(), 7);
        assert_eq!(ctx.vote.len(), 6);
        assert!(ctx.echoed());
        assert!(ctx.voted());

        receive_threshold_vote_message_for_deliver(
            &mut broadcaster,
            &block,
            *peers.get(8).unwrap(),
        );
    }

    fn receive_threshold_vote_message_for_deliver(
        broadcaster: &mut Broadcaster,
        block: &Block,
        peer_id: PeerId,
    ) {
        let rb_msg = RawRbMsg::new(block.clone(), PeerId::random());
        let rb_msg = rb_msg.vote_reply(peer_id, block.clone());

        let response = handle_double(broadcaster, &rb_msg);

        assert_matches!(response, BroadcastResponse::Deliver(_));
    }

    fn receive_nr_of_echo_messages_below_vote_threshold(
        broadcaster: &mut Broadcaster,
        block: &Block,
        peers: &[PeerId],
    ) {
        for peer_id in peers {
            let rb_msg = RawRbMsg::new(block.clone(), *peer_id);

            let response = handle_double(broadcaster, &rb_msg);

            assert_matches!(response, BroadcastResponse::Drop(_));
        }
    }

    fn receive_nr_of_vote_messages_below_deliver_threshold(
        broadcaster: &mut Broadcaster,
        block: &Block,
        peers: &[PeerId],
    ) {
        for peer_id in peers {
            let rb_msg = RawRbMsg::new(block.clone(), PeerId::random());
            let rb_msg = rb_msg.vote_reply(*peer_id, block.clone());

            let response = handle_double(broadcaster, &rb_msg);
            assert_matches!(response, BroadcastResponse::Drop(_));
        }
    }

    fn receive_echo_first_message(
        broadcaster: &mut Broadcaster,
        block: &Block,
        block_creator: PeerId,
    ) {
        let rb_msg = RawRbMsg::new(block.clone(), block_creator);
        let response = handle_double(broadcaster, &rb_msg);

        assert_matches!(
            response,
            BroadcastResponse::Broadcast(RawRbMsg {
                id: _,
                request_id: _,
                original_sender: _,
                timestamp: _,
                message_type: broadcast::MessageType::Echo(_),
            })
        );
    }

    fn receive_echo_threshold_message(
        broadcaster: &mut Broadcaster,
        block: &Block,
        peer_id: PeerId,
    ) {
        let rb_msg = RawRbMsg::new(block.clone(), peer_id);

        let response = handle_double(broadcaster, &rb_msg);
        assert_matches!(
            response,
            BroadcastResponse::Broadcast(RawRbMsg {
                id: _,
                request_id: _,
                original_sender: _,
                timestamp: _,
                message_type: broadcast::MessageType::Vote(_),
            })
        );
    }

    fn create_block(block_creator_peer_id: PeerId) -> (Hash, Block) {
        let header = RawBlockHeader::new(block_creator_peer_id, 0);
        let raw_block = RawBlock::new(header, vec![]);
        let block_hash = raw_block.hash_with_default_hasher().unwrap();
        let block = Block::new(raw_block, block_hash);
        (block_hash, block)
    }

    //make sure that duplicate messages doesn't have impact
    fn handle_double(broadcaster: &mut Broadcaster, rb_msg: &RawRbMsg) -> BroadcastResponse {
        let response = broadcaster.handle(rb_msg).unwrap();
        broadcaster.handle(rb_msg).unwrap();
        response
    }
}
