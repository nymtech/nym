// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::controller::error::DkgError;
use crate::coconut::dkg::key_derivation::{
    verification_key_finalization, verification_key_validation,
};
use crate::coconut::dkg::state::{ConsistentState, PersistentState, State};
use crate::coconut::keys::KeyPair as CoconutKeyPair;
use crate::nyxd;
use crate::support::config;
use anyhow::{bail, Result};
use nym_coconut_dkg_common::types::{Epoch, EpochId, EpochState};
use nym_crypto::asymmetric::identity;
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use nym_task::{TaskClient, TaskManager};
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::interval;

mod error;
pub(crate) mod keys;

pub(crate) struct DkgController<R = OsRng> {
    pub(crate) dkg_client: DkgClient,
    pub(crate) coconut_key_path: PathBuf,
    pub(crate) state: State,
    pub(super) rng: R,
    polling_rate: Duration,
}

impl<R: RngCore + CryptoRng + Clone> DkgController<R> {
    pub(crate) fn new(
        config: &config::CoconutSigner,
        nyxd_client: nyxd::Client,
        coconut_keypair: CoconutKeyPair,
        dkg_keypair: DkgKeyPair,
        identity_key: identity::PublicKey,
        rng: R,
    ) -> Result<Self> {
        let Some(announce_address) = &config.announce_address else {
            bail!("can't start a DKG controller without specifying an announce address!")
        };

        let persistent_state =
            PersistentState::load_from_file(&config.storage_paths.dkg_persistent_state_path)
                .unwrap_or_default();

        Ok(DkgController {
            dkg_client: DkgClient::new(nyxd_client),
            coconut_key_path: config.storage_paths.coconut_key_path.clone(),
            state: State::new(
                config.storage_paths.dkg_persistent_state_path.clone(),
                persistent_state,
                announce_address.clone(),
                dkg_keypair,
                identity_key,
                coconut_keypair,
            ),
            rng,
            polling_rate: config.debug.dkg_contract_polling_rate,
        })
    }

    fn persist_state(&self) -> Result<(), DkgError> {
        // if !self.state.coconut_keypair_is_some().await {
        //     // Delete the files just in case the process is killed before the new keys are generated
        //     std::fs::remove_file(&self.secret_key_path).ok();
        //     std::fs::remove_file(&self.verification_key_path).ok();
        // }
        let persistent_state = PersistentState::from(&self.state);
        let save_path = self.state.persistent_state_path();
        persistent_state.save_to_file(save_path).map_err(|source| {
            DkgError::StatePersistenceFailure {
                path: save_path.to_path_buf(),
                source,
            }
        })
    }

    async fn current_epoch(&self) -> Result<Epoch, DkgError> {
        self.dkg_client
            .get_current_epoch()
            .await
            .map_err(|source| DkgError::EpochQueryFailure { source })
    }

    async fn ensure_group_member(&self) -> Result<(), DkgError> {
        let membership_response = self
            .dkg_client
            .group_member()
            .await
            .map_err(|source| DkgError::GroupQueryFailure { source })?;

        debug!("CW4 membership response: {membership_response:?}");

        // make sure we are a voting member, i.e. have a non-zero weight
        if let Some(weight) = membership_response.weight {
            if weight == 0 {
                return Err(DkgError::NotInGroup);
            }
        } else {
            return Err(DkgError::NotInGroup);
        }

        Ok(())
    }

    async fn ensure_state_consistency(&self) -> Result<(), DkgError> {
        todo!()
        // if let Err(err) = self.state.is_consistent(epoch.state).await {
        //     warn!("Epoch state is corrupted - {err}. Awaiting for a DKG restart.");
        //     return;
        // }
    }

    async fn handle_awaiting_initialisation(&mut self) -> Result<(), DkgError> {
        info!("DKG hasn't been initialised yet - nothing to do");
        Ok(())
    }

    async fn handle_key_submission(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: public key submission (resharing: {resharing})");
        self.public_key_submission(epoch_id, resharing)
            .await
            .map_err(|source| DkgError::PublicKeySubmissionFailure { source })
    }

    async fn handle_dealing_exchange(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: dealing exchange (resharing: {resharing})");
        self.dealing_exchange(epoch_id, resharing)
            .await
            .map_err(|source| DkgError::DealingExchangeFailure { source })
    }

    async fn handle_verification_key_submission(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: verification key submission (resharing: {resharing})");
        self.verification_key_submission(epoch_id, resharing)
            .await
            .map_err(|source| DkgError::VerificationKeySubmissionFailure { source })
    }

    async fn handle_verification_key_validation(
        &mut self,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: verification key validation (resharing: {resharing})");

        verification_key_validation(&self.dkg_client, &mut self.state, resharing)
            .await
            .map_err(|source| DkgError::VerificationKeyValidationFailure { source })
    }

    async fn handle_verification_key_finalization(
        &mut self,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: verification key finalization (resharing: {resharing})");

        verification_key_finalization(&self.dkg_client, &mut self.state, resharing)
            .await
            .map_err(|source| DkgError::VerificationKeyFinalizationFailure { source })
    }

    async fn handle_in_progress(&mut self) -> Result<(), DkgError> {
        debug!("DKG: epoch in progress");

        self.state.set_was_in_progress();
        Ok(())
    }

    async fn try_advance_dkg_state(&mut self) -> Result<(), DkgError> {
        // We try advancing the epoch state, on a best-effort basis
        info!("DKG: Trying to advance the epoch");
        self.dkg_client
            .advance_epoch_state()
            .await
            .map_err(|source| DkgError::StateAdvancementFailure { source })
    }

    pub(crate) async fn handle_epoch_state(&mut self) -> Result<(), DkgError> {
        self.ensure_state_consistency().await?;
        self.ensure_group_member().await?;

        // make sure to always persist our state before continuing in case of failures
        self.persist_state()?;

        let epoch = self.current_epoch().await?;

        match epoch.state {
            EpochState::WaitingInitialisation => self.handle_awaiting_initialisation().await?,
            EpochState::PublicKeySubmission { resharing } => {
                self.handle_key_submission(epoch.epoch_id, resharing)
                    .await?
            }
            EpochState::DealingExchange { resharing } => {
                self.handle_dealing_exchange(epoch.epoch_id, resharing)
                    .await?
            }
            EpochState::VerificationKeySubmission { resharing } => {
                self.handle_verification_key_submission(epoch.epoch_id, resharing)
                    .await?
            }
            EpochState::VerificationKeyValidation { resharing } => {
                self.handle_verification_key_validation(resharing).await?
            }
            EpochState::VerificationKeyFinalization { resharing } => {
                self.handle_verification_key_finalization(resharing).await?
            }
            // Just wait, in case we need to redo dkg at some point
            EpochState::InProgress => self.handle_in_progress().await?,
        };

        // persist the state after the successful update
        // (sure, we might be doing this unnecessarily for each "InProgress",
        // but that's just one write every polling interval, which in the grand scheme of things is nothing
        self.persist_state()?;

        if let Some(epoch_finish) = epoch.finish_timestamp {
            let now = OffsetDateTime::now_utc();
            if now.unix_timestamp() > epoch_finish.seconds() as i64 {
                // TODO: make sure to not overload validator in case its running slow
                // i.e. send it once at most every X seconds
                self.try_advance_dkg_state().await?
            }
        }

        Ok(())
    }

    pub(crate) async fn run(mut self, mut shutdown: TaskClient) {
        let mut interval = interval(self.polling_rate);
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(err) = self.handle_epoch_state().await {
                        error!("failed to update the DKG state: {err}")
                    }
                }
                _ = shutdown.recv() => {
                    trace!("DkgController: Received shutdown");
                }
            }
        }
    }

    pub(crate) fn start(
        config: &config::CoconutSigner,
        nyxd_client: nyxd::Client,
        coconut_keypair: CoconutKeyPair,
        dkg_bte_keypair: DkgKeyPair,
        identity_key: identity::PublicKey,
        rng: R,
        shutdown: &TaskManager,
    ) -> Result<()>
    where
        R: Sync + Send + 'static,
    {
        let shutdown_listener = shutdown.subscribe();
        let dkg_controller = DkgController::new(
            config,
            nyxd_client,
            coconut_keypair,
            dkg_bte_keypair,
            identity_key,
            rng,
        )?;
        tokio::spawn(async move { dkg_controller.run(shutdown_listener).await });
        Ok(())
    }
}

#[cfg(test)]
impl DkgController {
    pub(crate) fn test_mock(dkg_client: DkgClient, state: State) -> DkgController {
        DkgController {
            dkg_client,
            coconut_key_path: Default::default(),
            state,
            rng: OsRng,
            polling_rate: Default::default(),
        }
    }

    pub(crate) fn test_mock_new(
        rng: rand_chacha::ChaCha20Rng,
        dkg_client: DkgClient,
        state: State,
        coconut_key_path: PathBuf,
    ) -> DkgController<rand_chacha::ChaCha20Rng> {
        DkgController {
            dkg_client,
            coconut_key_path,
            state,
            rng,
            polling_rate: Default::default(),
        }
    }
}
