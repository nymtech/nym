// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::client::Client;
use crate::coconut::dkg::publisher::Publisher;
use rand::rngs::OsRng;
use task::ShutdownListener;

pub(crate) mod publisher;

pub(crate) struct DkgController {
    publisher: Publisher,
    decryption_key: dkg::bte::DecryptionKey,
    public_key: dkg::bte::PublicKeyWithProof,
}

impl DkgController {
    pub(crate) fn new<C>(nymd_client: C) -> Self
    where
        C: Client + Send + Sync + 'static,
    {
        let publisher = Publisher::new(nymd_client);
        let mut rng = OsRng;
        let dkg_params = dkg::bte::setup();
        let (decryption_key, public_key) = dkg::bte::keygen(&dkg_params, &mut rng);
        DkgController {
            publisher,
            decryption_key,
            public_key,
        }
    }

    pub(crate) async fn run(&self, mut shutdown: ShutdownListener) {
        let bte_key = bs58::encode(&self.public_key.to_bytes()).into_string();
        let index = self
            .publisher
            .register_dealer(bte_key)
            .await
            .expect("Could not register dealer in dkg protocol");
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = shutdown.recv() => {
                    trace!("DkgController: Received shutdown");
                }
            }
        }
    }
}
