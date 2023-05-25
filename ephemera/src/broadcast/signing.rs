use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::Arc;

use log::trace;
use lru::LruCache;

use crate::{
    block::types::block::{Block, RawBlock},
    crypto::Keypair,
    utilities::{codec::Encode, crypto::Certificate, crypto::EphemeraPublicKey, hash::Hash},
};

pub(crate) struct BlockSigner {
    /// All signatures of the last blocks that we received from the network(+ our own)
    verified_signatures: LruCache<Hash, HashSet<Certificate>>,
    /// Our own keypair
    signing_keypair: Arc<Keypair>,
}

impl BlockSigner {
    pub fn new(keypair: Arc<Keypair>) -> Self {
        Self {
            verified_signatures: LruCache::new(NonZeroUsize::new(1000).unwrap()),
            signing_keypair: keypair,
        }
    }

    pub(crate) fn get_block_certificates(
        &mut self,
        block_id: &Hash,
    ) -> Option<&HashSet<Certificate>> {
        self.verified_signatures.get(block_id)
    }

    pub(crate) fn sign_block(&mut self, block: &Block, hash: &Hash) -> anyhow::Result<Certificate> {
        trace!("Signing block: {:?}", block);

        let certificate = block.sign(self.signing_keypair.as_ref())?;
        self.add_certificate(hash, certificate.clone());
        Ok(certificate)
    }

    /// This verification is part of reliable broadcast and verifies only the
    /// signature of the sender.
    pub(crate) fn verify_block(
        &mut self,
        block: &Block,
        certificate: &Certificate,
    ) -> anyhow::Result<()> {
        trace!("Verifying block: {block:?} against certificate {certificate:?}");

        let raw_block: RawBlock = (*block).clone().into();
        let raw_block = raw_block.encode()?;

        if certificate
            .public_key
            .verify(&raw_block, &certificate.signature)
        {
            self.add_certificate(&block.header.hash, certificate.clone());
            Ok(())
        } else {
            anyhow::bail!("Invalid block certificate");
        }
    }

    fn add_certificate(&mut self, hash: &Hash, certificate: Certificate) {
        trace!("Adding certificate to block: {}", hash);
        self.verified_signatures
            .get_or_insert_mut(*hash, HashSet::new)
            .insert(certificate);
    }
}

#[cfg(test)]
mod test {
    use crate::block::types::block::RawBlockHeader;
    use crate::block::types::message::{EphemeraMessage, RawEphemeraMessage};
    use crate::crypto::EphemeraKeypair;
    use crate::peer::ToPeerId;

    use super::*;

    #[test]
    fn test_sign_verify_block_ok() {
        let mut signer = BlockSigner::new(Arc::new(Keypair::generate(None)));

        let message_signing_keypair = Keypair::generate(None);

        let block = new_block(&message_signing_keypair, "label1");
        let hash = block.hash_with_default_hasher().unwrap();

        let certificate = signer.sign_block(&block, &hash).unwrap();

        assert!(signer.verify_block(&block, &certificate).is_ok());
    }

    #[test]
    fn test_sign_signatures_cached_correctly() {
        let mut signer = BlockSigner::new(Arc::new(Keypair::generate(None)));

        let block = new_block(&Keypair::generate(None), "label1");
        let hash = block.hash_with_default_hasher().unwrap();

        //Signed by node 1
        let certificate1 = block.sign(&Keypair::generate(None)).unwrap();
        signer.verify_block(&block, &certificate1).unwrap();
        //Signed by node 2
        let certificate2 = block.sign(&Keypair::generate(None)).unwrap();
        signer.verify_block(&block, &certificate2).unwrap();

        let block_certificates = signer.get_block_certificates(&hash).unwrap();
        assert_eq!(block_certificates.len(), 2);
    }

    #[test]
    fn test_sign_verify_block_fail() {
        let mut signer = BlockSigner::new(Arc::new(Keypair::generate(None)));
        let message_signing_keypair = Keypair::generate(None);

        let block = new_block(&message_signing_keypair, "label1");
        let certificate = block.sign(&message_signing_keypair).unwrap();

        let modified_block = new_block(&message_signing_keypair, "label2");

        assert!(signer.verify_block(&modified_block, &certificate).is_err());
    }

    fn new_block(keypair: &Keypair, message_label: &str) -> Block {
        let peer_id = keypair.public_key().peer_id();

        let raw_ephemera_message =
            RawEphemeraMessage::new(message_label.to_string(), "payload".as_bytes().to_vec());

        let message_certificate = Certificate::prepare(keypair, &raw_ephemera_message).unwrap();
        let messages = vec![EphemeraMessage::new(
            raw_ephemera_message,
            message_certificate,
        )];

        let raw_block_header = RawBlockHeader::new(peer_id, 0);
        let raw_block = RawBlock::new(raw_block_header, messages);

        let block_hash = raw_block
            .hash_with_default_hasher()
            .expect("Hashing failed");

        Block::new(raw_block, block_hash)
    }
}
