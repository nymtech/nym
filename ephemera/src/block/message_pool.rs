//! Message pool for Ephemera messages
//!
//! It stores pending Ephemera messages which will be added to a future block.
//! It doesn't have any other logic than just storing messages.
//!
//! It's up to the user provided [`crate::ephemera_api::Application::check_tx`] to decide which messages to include.

use std::collections::HashMap;

use log::{trace, warn};

use crate::block::types::message::EphemeraMessage;
use crate::utilities::hash::Hash;

pub(crate) struct MessagePool {
    pending_messages: HashMap<Hash, EphemeraMessage>,
}

impl MessagePool {
    pub(super) fn new() -> Self {
        Self {
            pending_messages: HashMap::default(),
        }
    }

    pub(crate) fn contains(&self, hash: &Hash) -> bool {
        self.pending_messages.contains_key(hash)
    }

    pub(super) fn add_message(&mut self, msg: EphemeraMessage) -> anyhow::Result<()> {
        trace!("Adding message to pool: {:?}", msg);

        let msg_hash = msg.hash_with_default_hasher()?;

        self.pending_messages.insert(msg_hash, msg);

        trace!("Message pool size: {:?}", self.pending_messages.len());
        Ok(())
    }

    pub(super) fn remove_messages(&mut self, messages: &[EphemeraMessage]) -> anyhow::Result<()> {
        trace!(
            "Mempool size before removing messages {}",
            self.pending_messages.len()
        );
        for msg in messages {
            let hash = msg.hash_with_default_hasher()?;
            if self.pending_messages.remove(&hash).is_none() {
                warn!("Message not found in pool: {:?}", msg);
            }
        }
        trace!(
            "Mempool size after removing messages {}",
            self.pending_messages.len()
        );
        Ok(())
    }

    /// Returns a `Vec` of all `EphemeraMessage`s in the message pool.
    /// The message pool is not cleared.
    pub(super) fn get_messages(&self) -> Vec<EphemeraMessage> {
        self.pending_messages.values().cloned().collect()
    }
}

#[cfg(test)]
mod test {
    use crate::block::message_pool::MessagePool;
    use crate::block::types::message::EphemeraMessage;
    use crate::crypto::{EphemeraKeypair, Keypair};
    use crate::ephemera_api::RawApiEphemeraMessage;

    #[test]
    fn test_add_remove() {
        let keypair = Keypair::generate(None);

        let message = RawApiEphemeraMessage::new("test".to_string(), vec![1, 2, 3]);
        let signed_message = message.sign(&keypair).expect("Failed to sign message");
        let signed_message: EphemeraMessage = signed_message.into();

        let mut pool = MessagePool::new();
        pool.add_message(signed_message.clone()).unwrap();
        pool.remove_messages(&[signed_message]).unwrap();

        assert_eq!(pool.get_messages().len(), 0);
    }
}
