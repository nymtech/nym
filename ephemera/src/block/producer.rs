use crate::block::{
    types::block::{Block, RawBlock, RawBlockHeader},
    types::message::EphemeraMessage,
};
use crate::peer::PeerId;
use log::trace;

pub(crate) struct BlockProducer {
    pub(crate) peer_id: PeerId,
}

impl BlockProducer {
    pub(super) fn new(peer_id: PeerId) -> Self {
        Self { peer_id }
    }

    pub(super) fn create_block(
        &mut self,
        height: u64,
        pending_messages: Vec<EphemeraMessage>,
    ) -> anyhow::Result<Block> {
        trace!("Pending messages for new block: {:?}", pending_messages);
        let block = self.new_block(height, pending_messages)?;
        Ok(block)
    }

    fn new_block(&self, height: u64, mut messages: Vec<EphemeraMessage>) -> anyhow::Result<Block> {
        //Ordering is fundamental for block hash. Simple sort is fine for now.
        messages.sort();

        let raw_header = RawBlockHeader::new(self.peer_id, height);
        let raw_block = RawBlock::new(raw_header, messages);

        //Better idea is probably combine header hash with Merkle tree root hash
        let block_hash = raw_block.hash_with_default_hasher()?;

        let block = Block::new(raw_block, block_hash);
        Ok(block)
    }
}

#[cfg(test)]
mod test {
    use crate::crypto::{EphemeraKeypair, Keypair};
    use crate::ephemera_api::RawApiEphemeraMessage;
    use std::cmp::Ordering;

    use super::*;

    #[test]
    fn test_produce_block() {
        let peer_id = PeerId::random();

        let mut block_producer = BlockProducer::new(peer_id);

        let message = RawApiEphemeraMessage::new("test".to_string(), vec![1, 2, 3]);
        let signed_message = message
            .sign(&Keypair::generate(None))
            .expect("Failed to sign message");
        let signed_message1: EphemeraMessage = signed_message.into();

        let message = RawApiEphemeraMessage::new("test".to_string(), vec![1, 2, 3]);
        let signed_message = message
            .sign(&Keypair::generate(None))
            .expect("Failed to sign message");
        let signed_message2: EphemeraMessage = signed_message.into();

        let messages = vec![signed_message1.clone(), signed_message2.clone()];

        let block = block_producer.create_block(1, messages).unwrap();

        assert_eq!(block.header.height, 1);
        assert_eq!(block.header.creator, peer_id);
        assert_eq!(block.messages.len(), 2);

        //Nondeterministic because of timestamp
        match signed_message1.cmp(&signed_message2) {
            Ordering::Less => {
                assert_eq!(block.messages[0], signed_message1);
                assert_eq!(block.messages[1], signed_message2);
            }
            Ordering::Greater => {
                assert_eq!(block.messages[0], signed_message2);
                assert_eq!(block.messages[1], signed_message1);
            }

            Ordering::Equal => {
                panic!("Messages are equal");
            }
        }
    }
}
