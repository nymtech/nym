use std::fmt::{Debug, Display};

use serde::{Deserialize, Serialize};

use crate::utilities::merkle::MerkleTree;
use crate::{
    block::types::message::EphemeraMessage,
    codec::{Decode, Encode},
    crypto::Keypair,
    peer::PeerId,
    utilities::{
        codec::{Codec, DecodingError, EncodingError, EphemeraCodec},
        crypto::Certificate,
        hash::{EphemeraHash, EphemeraHasher},
        hash::{Hash, Hasher},
        time::EphemeraTime,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct BlockHeader {
    pub(crate) timestamp: u64,
    pub(crate) creator: PeerId,
    pub(crate) height: u64,
    pub(crate) hash: Hash,
}

impl BlockHeader {
    pub(crate) fn new(raw_header: &RawBlockHeader, hash: Hash) -> Self {
        Self {
            timestamp: raw_header.timestamp,
            creator: raw_header.creator,
            height: raw_header.height,
            hash,
        }
    }
}

impl Display for BlockHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hash = &self.hash;
        let time = self.timestamp;
        let creator = &self.creator;
        let height = self.height;
        write!(
            f,
            "hash: {hash}, timestamp: {time}, creator: {creator}, height: {height}",
        )
    }
}

impl Encode for BlockHeader {
    fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        Codec::encode(&self)
    }
}

impl Decode for BlockHeader {
    type Output = Self;

    fn decode(bytes: &[u8]) -> Result<Self::Output, DecodingError> {
        Codec::decode(bytes)
    }
}

impl EphemeraHash for BlockHeader {
    fn hash<H: EphemeraHasher>(&self, state: &mut H) -> anyhow::Result<()> {
        let bytes = Codec::encode(&self)?;
        state.update(&bytes);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct RawBlockHeader {
    pub(crate) timestamp: u64,
    pub(crate) creator: PeerId,
    pub(crate) height: u64,
}

impl RawBlockHeader {
    pub(crate) fn new(creator: PeerId, height: u64) -> Self {
        Self {
            timestamp: EphemeraTime::now(),
            creator,
            height,
        }
    }

    pub(crate) fn hash_with_default_hasher(&self) -> anyhow::Result<Hash> {
        let mut hasher = Hasher::default();
        self.hash(&mut hasher)?;
        let header_hash = hasher.finish().into();
        Ok(header_hash)
    }
}

impl Display for RawBlockHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let creator = &self.creator;
        let height = self.height;
        write!(f, "creator: {creator}, height: {height}",)
    }
}

impl From<BlockHeader> for RawBlockHeader {
    fn from(block_header: BlockHeader) -> Self {
        Self {
            timestamp: block_header.timestamp,
            creator: block_header.creator,
            height: block_header.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct Block {
    pub(crate) header: BlockHeader,
    pub(crate) messages: Vec<EphemeraMessage>,
}

impl Block {
    pub(crate) fn new(raw_block: RawBlock, block_hash: Hash) -> Self {
        let header = BlockHeader::new(&raw_block.header, block_hash);
        Self {
            header,
            messages: raw_block.messages,
        }
    }

    pub(crate) fn get_hash(&self) -> Hash {
        self.header.hash
    }

    pub(crate) fn get_height(&self) -> u64 {
        self.header.height
    }

    pub(crate) fn new_genesis_block(creator: PeerId) -> Self {
        let mut block = Self {
            header: BlockHeader {
                timestamp: EphemeraTime::now(),
                creator,
                height: 0,
                hash: Hash::new([0; 32]),
            },
            messages: Vec::new(),
        };

        let hash = block
            .hash_with_default_hasher()
            .expect("Failed to hash genesis block");
        block.header.hash = hash;
        block
    }

    pub(crate) fn sign(&self, keypair: &Keypair) -> anyhow::Result<Certificate> {
        let raw_block: RawBlock = self.clone().into();
        let certificate = Certificate::prepare(keypair, &raw_block)?;
        Ok(certificate)
    }

    pub(crate) fn verify(&self, certificate: &Certificate) -> anyhow::Result<bool> {
        let raw_block: RawBlock = self.clone().into();
        certificate.verify(&raw_block)
    }

    pub(crate) fn hash_with_default_hasher(&self) -> anyhow::Result<Hash> {
        let raw_block: RawBlock = self.clone().into();
        raw_block.hash_with_default_hasher()
    }

    pub(crate) fn merkle_tree(&self) -> anyhow::Result<MerkleTree> {
        merkle_tree(&self.messages)
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let header = &self.header;
        write!(f, "{header}, nr of messages: {}", self.messages.len())
    }
}

impl Encode for Block {
    fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        Codec::encode(&self)
    }
}

impl Decode for Block {
    type Output = Block;

    fn decode(bytes: &[u8]) -> Result<Self::Output, DecodingError> {
        Codec::decode(bytes)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct RawBlock {
    pub(crate) header: RawBlockHeader,
    pub(crate) messages: Vec<EphemeraMessage>,
}

impl RawBlock {
    pub(crate) fn new(header: RawBlockHeader, messages: Vec<EphemeraMessage>) -> Self {
        Self { header, messages }
    }

    pub(crate) fn hash_with_default_hasher(&self) -> anyhow::Result<Hash> {
        let header_hash = self.header.hash_with_default_hasher()?;
        let merkle_root = merkle_tree(&self.messages)?.root_hash();
        let block_hash = Hasher::digest(&[header_hash.inner(), merkle_root.inner()].concat());
        Ok(block_hash.into())
    }
}

impl From<Block> for RawBlock {
    fn from(block: Block) -> Self {
        Self {
            header: block.header.into(),
            messages: block.messages,
        }
    }
}

impl Encode for RawBlockHeader {
    fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        Codec::encode(&self)
    }
}

impl Decode for RawBlockHeader {
    type Output = RawBlockHeader;

    fn decode(bytes: &[u8]) -> Result<Self::Output, DecodingError> {
        Codec::decode(bytes)
    }
}

impl Encode for RawBlock {
    fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        Codec::encode(&self)
    }
}

impl Decode for RawBlock {
    type Output = RawBlock;

    fn decode(bytes: &[u8]) -> Result<Self::Output, DecodingError> {
        Codec::decode(bytes)
    }
}

impl EphemeraHash for RawBlockHeader {
    fn hash<H: EphemeraHasher>(&self, state: &mut H) -> anyhow::Result<()> {
        state.update(&self.encode()?);
        Ok(())
    }
}

impl EphemeraHash for RawBlock {
    fn hash<H: EphemeraHasher>(&self, state: &mut H) -> anyhow::Result<()> {
        self.header.hash(state)?;
        for message in &self.messages {
            message.hash(state)?;
        }
        Ok(())
    }
}

pub(crate) fn merkle_tree(messages: &[EphemeraMessage]) -> anyhow::Result<MerkleTree> {
    let message_hashes = messages
        .iter()
        .map(EphemeraMessage::hash_with_default_hasher)
        .collect::<anyhow::Result<Vec<Hash>>>()?;
    let merkle_tree = MerkleTree::build_tree(&message_hashes);
    Ok(merkle_tree)
}

#[cfg(test)]
mod test {
    use crate::block::types::message::RawEphemeraMessage;
    use crate::crypto::EphemeraKeypair;

    use super::*;

    #[test]
    fn test_block_hash_no_messages() {
        let block = Block::new_genesis_block(PeerId::random());
        let block_hash = block.hash_with_default_hasher().unwrap();
        assert_eq!(block_hash, block.get_hash());
    }

    #[test]
    fn test_block_hash_with_messages() {
        let messages = create_ephemera_messages(10);
        let message_hashes = messages
            .iter()
            .map(EphemeraMessage::hash_with_default_hasher)
            .collect::<anyhow::Result<Vec<Hash>>>()
            .unwrap();

        let raw_block = RawBlock::new(RawBlockHeader::new(PeerId::random(), 0), messages);
        let block_hash = raw_block.hash_with_default_hasher().unwrap();

        let header_hash = raw_block.header.hash_with_default_hasher().unwrap();
        let merkle_root = MerkleTree::build_tree(&message_hashes).root_hash();
        let expected_block_hash =
            Hasher::digest(&[header_hash.inner(), merkle_root.inner()].concat());

        assert_eq!(block_hash, expected_block_hash.into());
    }

    fn create_ephemera_messages(n: usize) -> Vec<EphemeraMessage> {
        let keypair = Keypair::generate(None);
        let mut messages = Vec::new();
        for i in 0..n {
            let label = format!("test {i}",);
            let message = RawEphemeraMessage::new(label, vec![0; 32]);
            let certificate = Certificate::prepare(&keypair, &message).unwrap();
            let ephemera_message = EphemeraMessage::new(message, certificate);
            messages.push(ephemera_message);
        }
        messages
    }
}
