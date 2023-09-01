use std::sync::Arc;

use crate::crypto::{EphemeraKeypair, Keypair};

pub(crate) struct KeyManager;

impl KeyManager {
    pub(crate) fn read_keypair_from_str(private_key: &str) -> anyhow::Result<Arc<Keypair>> {
        let keypair: Arc<Keypair> = Keypair::from_base58(private_key)
            .map_err(|e| anyhow::anyhow!("Failed to parse private key from config. Error: {e:?}"))?
            .into();
        Ok(keypair)
    }
}
