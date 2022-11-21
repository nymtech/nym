// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::client::Client;
use crate::coconut::dkg::publisher::Publisher;
use crate::Config;
use anyhow::Result;
use dkg::bte::keys::KeyPair;
use rand::rngs::OsRng;
use task::ShutdownListener;

pub(crate) mod publisher;

pub(crate) fn init_keypair(config: &Config) -> Result<()> {
    let mut rng = OsRng;
    let dkg_params = dkg::bte::setup();
    let kp = KeyPair::new(&dkg_params, &mut rng);
    pemstore::store_keypair(
        &kp,
        &pemstore::KeyPairPath::new(
            config.decryption_key_path(),
            config.public_key_with_proof_path(),
        ),
    )?;
    Ok(())
}

pub(crate) struct DkgController {
    publisher: Publisher,
    keypair: KeyPair,
}

impl DkgController {
    pub(crate) fn new<C>(config: &Config, nymd_client: C) -> Result<Self>
    where
        C: Client + Send + Sync + 'static,
    {
        let publisher = Publisher::new(nymd_client);
        let keypair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
            config.decryption_key_path(),
            config.public_key_with_proof_path(),
        ))?;
        Ok(DkgController { publisher, keypair })
    }

    pub(crate) async fn run(&self, mut shutdown: ShutdownListener) {
        let bte_key = bs58::encode(&self.keypair.public_key().to_bytes()).into_string();
        let index = if let Some(details) = self
            .publisher
            .get_self_registered_dealer_details()
            .await
            .expect("Could not query for dealer details")
            .details
        {
            details.assigned_index
        } else {
            self.publisher
                .register_dealer(bte_key)
                .await
                .expect("Could not register dealer in dkg protocol")
        };
        info!("Starting dkg protocol with index {}", index);
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = shutdown.recv() => {
                    trace!("DkgController: Received shutdown");
                }
            }
        }
    }
}
