// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::{ConsistentState, PersistentState, State};
use crate::coconut::dkg::verification_key::{
    verification_key_finalization, verification_key_validation,
};
use crate::coconut::dkg::{
    dealing::dealing_exchange, public_key::public_key_submission,
    verification_key::verification_key_submission,
};
use crate::coconut::keypair::KeyPair as CoconutKeyPair;
use crate::{nyxd_client, Config};
use anyhow::Result;
use coconut_dkg_common::types::EpochState;
use dkg::bte::keys::KeyPair as DkgKeyPair;
use rand::rngs::OsRng;
use rand::RngCore;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use task::TaskClient;
use tokio::time::interval;
use validator_client::nyxd::SigningNyxdClient;

pub(crate) fn init_keypair(config: &Config) -> Result<()> {
    let mut rng = OsRng;
    let dkg_params = dkg::bte::setup();
    let kp = DkgKeyPair::new(&dkg_params, &mut rng);
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
    secret_key_path: PathBuf,
    verification_key_path: PathBuf,
    state: State,
    rng: R,
    polling_rate: Duration,
}

impl<R: RngCore + Clone> DkgController<R> {
    pub(crate) async fn new(
        config: &Config,
        nyxd_client: nyxd_client::Client<SigningNyxdClient>,
        coconut_keypair: CoconutKeyPair,
        rng: R,
    ) -> Result<Self> {
        let dkg_keypair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
            config.decryption_key_path(),
            config.public_key_with_proof_path(),
        ))?;
        if let Ok(coconut_keypair_value) = pemstore::load_keypair(&pemstore::KeyPairPath::new(
            config.secret_key_path(),
            config.verification_key_path(),
        )) {
            coconut_keypair.set(coconut_keypair_value).await;
        }
        let persistent_state =
            PersistentState::load_from_file(config.persistent_state_path()).unwrap_or_default();

        Ok(DkgController {
            dkg_client: DkgClient::new(nyxd_client),
            secret_key_path: config.secret_key_path(),
            verification_key_path: config.verification_key_path(),
            state: State::new(
                config.persistent_state_path(),
                persistent_state,
                config.get_announce_address(),
                dkg_keypair,
                coconut_keypair,
            ),
            rng,
            polling_rate: config.get_dkg_contract_polling_rate(),
        })
    }

    pub(crate) async fn handle_epoch_state(&mut self) {
        match self.dkg_client.get_current_epoch().await {
            Err(e) => warn!("Could not get current epoch state {}", e),
            Ok(epoch) => {
                if let Err(e) = self.state.is_consistent(epoch.state).await {
                    error!(
                        "Epoch state is corrupted - {}, the process should be terminated",
                        e
                    );
                }
                let ret = match epoch.state {
                    EpochState::PublicKeySubmission => {
                        public_key_submission(&self.dkg_client, &mut self.state).await
                    }
                    EpochState::DealingExchange => {
                        dealing_exchange(&self.dkg_client, &mut self.state, self.rng.clone()).await
                    }
                    EpochState::VerificationKeySubmission => {
                        let keypair_path = pemstore::KeyPairPath::new(
                            self.secret_key_path.clone(),
                            self.verification_key_path.clone(),
                        );
                        verification_key_submission(
                            &self.dkg_client,
                            &mut self.state,
                            &keypair_path,
                        )
                        .await
                    }
                    EpochState::VerificationKeyValidation => {
                        verification_key_validation(&self.dkg_client, &mut self.state).await
                    }
                    EpochState::VerificationKeyFinalization => {
                        verification_key_finalization(&self.dkg_client, &mut self.state).await
                    }
                    // Just wait, in case we need to redo dkg at some point
                    EpochState::InProgress => Ok(()),
                };
                if let Err(e) = ret {
                    warn!("Could not handle this iteration for the epoch state: {}", e);
                } else if epoch.state != EpochState::InProgress {
                    let persistent_state = PersistentState::from(&self.state);
                    if let Err(e) =
                        persistent_state.save_to_file(self.state.persistent_state_path())
                    {
                        warn!("Could not backup the state for this iteration: {}", e);
                    }
                }
                if let Ok(current_timestamp) =
                    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                {
                    if current_timestamp.as_secs() >= epoch.finish_timestamp.seconds() {
                        // We try advancing the epoch state, on a best-effort basis
                        info!("Trying to advance the epoch");
                        self.dkg_client.advance_epoch_state().await.ok();
                    }
                }
            }
        }
    }

    pub(crate) async fn run(mut self, mut shutdown: TaskClient) {
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
