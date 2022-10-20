// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::dealing::dealing_exchange;
use crate::coconut::dkg::public_key::public_key_submission;
use crate::coconut::dkg::state::State;
use crate::{nymd_client, Config};
use anyhow::Result;
use coconut_dkg_common::types::EpochState;
use dkg::bte::keys::KeyPair;
use rand::rngs::OsRng;
use rand::RngCore;
use std::time::Duration;
use task::ShutdownListener;
use tokio::time::interval;
use validator_client::nymd::SigningNymdClient;

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

pub(crate) struct DkgController<R> {
    dkg_client: DkgClient,
    state: State,
    rng: R,
    polling_rate: Duration,
}

impl<R: RngCore> DkgController<R> {
    pub(crate) fn new(
        config: &Config,
        nymd_client: nymd_client::Client<SigningNymdClient>,
        rng: R,
    ) -> Result<Self> {
        let keypair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
            config.decryption_key_path(),
            config.public_key_with_proof_path(),
        ))?;

        Ok(DkgController {
            dkg_client: DkgClient::new(nymd_client),
            state: State::new(keypair),
            rng,
            polling_rate: config.get_dkg_contract_polling_rate(),
        })
    }

    async fn handle_epoch_state(&mut self) {
        match self.dkg_client.get_current_epoch_state().await {
            Err(e) => warn!("Could not get current epoch state {}", e),
            Ok(epoch_state) => {
                let ret = match epoch_state {
                    EpochState::PublicKeySubmission => {
                        public_key_submission(&self.dkg_client, &mut self.state).await
                    }
                    EpochState::DealingExchange => {
                        dealing_exchange(&self.dkg_client, &mut self.state, &mut self.rng).await
                    }
                    EpochState::ComplaintSubmission | EpochState::ComplaintVoting => {
                        trace!("Remains to be implemented using multisig contract");
                        Ok(())
                    }
                    _ => todo!(),
                };
                if let Err(e) = ret {
                    warn!("Could not handle this iteration for the epoch state: {}", e);
                }
            }
        }
    }

    pub(crate) async fn run(mut self, mut shutdown: ShutdownListener) {
        let mut interval = interval(self.polling_rate);
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = interval.tick() => self.handle_epoch_state().await,
                _ = shutdown.recv() => {
                    trace!("DkgController: Received shutdown");
                }
            }
        }
    }
}
