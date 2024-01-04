// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

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
use crate::nyxd;
use crate::support::config;
use anyhow::{bail, Result};
use nym_coconut_dkg_common::types::EpochState;
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use nym_task::{TaskClient, TaskManager};
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::time::interval;

pub(crate) fn init_keypair(config: &config::CoconutSigner) -> Result<()> {
    let mut rng = OsRng;
    let dkg_params = nym_dkg::bte::setup();
    let kp = DkgKeyPair::new(&dkg_params, &mut rng);
    nym_pemstore::store_keypair(
        &kp,
        &nym_pemstore::KeyPairPath::new(
            &config.storage_paths.decryption_key_path,
            &config.storage_paths.public_key_with_proof_path,
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

impl<R: RngCore + CryptoRng + Clone> DkgController<R> {
    pub(crate) async fn new(
        config: &config::CoconutSigner,
        nyxd_client: nyxd::Client,
        coconut_keypair: CoconutKeyPair,
        rng: R,
    ) -> Result<Self> {
        let Some(announce_address) = &config.announce_address else {
            bail!("can't start a DKG controller without specifying an announce address!")
        };

        let dkg_keypair = nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
            &config.storage_paths.decryption_key_path,
            &config.storage_paths.public_key_with_proof_path,
        ))?;
        if let Ok(coconut_keypair_value) =
            nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
                &config.storage_paths.secret_key_path,
                &config.storage_paths.verification_key_path,
            ))
        {
            coconut_keypair.set(Some(coconut_keypair_value)).await;
        }
        let persistent_state =
            PersistentState::load_from_file(&config.storage_paths.dkg_persistent_state_path)
                .unwrap_or_default();

        Ok(DkgController {
            dkg_client: DkgClient::new(nyxd_client),
            secret_key_path: config.storage_paths.secret_key_path.clone(),
            verification_key_path: config.storage_paths.verification_key_path.clone(),
            state: State::new(
                config.storage_paths.dkg_persistent_state_path.clone(),
                persistent_state,
                announce_address.clone(),
                dkg_keypair,
                coconut_keypair,
            ),
            rng,
            polling_rate: config.debug.dkg_contract_polling_rate,
        })
    }

    async fn dump_persistent_state(&self) {
        if !self.state.coconut_keypair_is_some().await {
            // Delete the files just in case the process is killed before the new keys are generated
            std::fs::remove_file(&self.secret_key_path).ok();
            std::fs::remove_file(&self.verification_key_path).ok();
        }
        let persistent_state = PersistentState::from(&self.state);
        if let Err(err) = persistent_state.save_to_file(self.state.persistent_state_path()) {
            warn!("Could not backup the state for this iteration: {err}");
        }
    }

    pub(crate) async fn handle_epoch_state(&mut self) {
        match self.dkg_client.get_current_epoch().await {
            Err(err) => warn!("Could not get current epoch state {err}"),
            Ok(epoch) => {
                if self
                    .dkg_client
                    .group_member()
                    .await
                    .map(|resp| resp.weight.is_none())
                    .unwrap_or(true)
                {
                    debug!("Not a member of the group, DKG won't be run");
                    return;
                }
                if let Err(err) = self.state.is_consistent(epoch.state).await {
                    debug!("Epoch state is corrupted - {err}. Awaiting for a DKG restart.");
                } else {
                    let ret = match epoch.state {
                        EpochState::PublicKeySubmission { resharing } => {
                            public_key_submission(&self.dkg_client, &mut self.state, resharing)
                                .await
                        }
                        EpochState::DealingExchange { resharing } => {
                            dealing_exchange(
                                &self.dkg_client,
                                &mut self.state,
                                self.rng.clone(),
                                resharing,
                            )
                            .await
                        }
                        EpochState::VerificationKeySubmission { resharing } => {
                            let keypair_path = nym_pemstore::KeyPairPath::new(
                                self.secret_key_path.clone(),
                                self.verification_key_path.clone(),
                            );
                            verification_key_submission(
                                &self.dkg_client,
                                &mut self.state,
                                epoch.epoch_id,
                                &keypair_path,
                                resharing,
                            )
                            .await
                        }
                        EpochState::VerificationKeyValidation { resharing } => {
                            verification_key_validation(
                                &self.dkg_client,
                                &mut self.state,
                                resharing,
                            )
                            .await
                        }
                        EpochState::VerificationKeyFinalization { resharing } => {
                            verification_key_finalization(
                                &self.dkg_client,
                                &mut self.state,
                                resharing,
                            )
                            .await
                        }
                        // Just wait, in case we need to redo dkg at some point
                        EpochState::InProgress => {
                            self.state.set_was_in_progress();
                            // We're dumping state here so that we don't do it uselessly during the
                            // long InProgress state
                            self.dump_persistent_state().await;
                            Ok(())
                        }
                    };
                    if let Err(err) = ret {
                        warn!("Could not handle this iteration for the epoch state: {err}");
                    } else if epoch.state != EpochState::InProgress {
                        self.dump_persistent_state().await;
                    }
                }
                if let Ok(current_timestamp) =
                    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                {
                    if current_timestamp.as_secs() >= epoch.finish_timestamp.seconds() {
                        // We try advancing the epoch state, on a best-effort basis
                        info!("DKG: Trying to advance the epoch");
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

    // TODO: can we make it non-async? it seems we'd have to modify `coconut_keypair.set(coconut_keypair_value)` in new
    // could we do it?
    pub(crate) async fn start(
        config: &config::CoconutSigner,
        nyxd_client: nyxd::Client,
        coconut_keypair: CoconutKeyPair,
        rng: R,
        shutdown: &TaskManager,
    ) -> Result<()>
    where
        R: Sync + Send + 'static,
    {
        let shutdown_listener = shutdown.subscribe();
        let dkg_controller = DkgController::new(config, nyxd_client, coconut_keypair, rng).await?;
        tokio::spawn(async move { dkg_controller.run(shutdown_listener).await });
        Ok(())
    }
}
