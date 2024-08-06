// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg::client::DkgClient;
use crate::ecash::dkg::controller::error::DkgError;
use crate::ecash::dkg::state::{PersistentState, State};
use crate::ecash::keys::KeyPair as CoconutKeyPair;
use crate::nyxd;
use crate::support::config;
use anyhow::{bail, Result};
use nym_coconut_dkg_common::types::{Epoch, EpochId, EpochState};
use nym_crypto::asymmetric::identity;
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use nym_task::{TaskClient, TaskManager};
use rand::rngs::OsRng;
use rand::{CryptoRng, Rng, RngCore};
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{interval, MissedTickBehavior};

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

        let persistent_state = PersistentState::load_from_file(
            &config.storage_paths.dkg_persistent_state_path,
        ).unwrap_or_else(|err| {
            warn!("could not load an existing persistent state from the file. a fresh state will be used: {err}");
            Default::default()
        });

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
            .map_err(|source| DkgError::PublicKeySubmissionFailure { source })?;
        self.persist_state()
    }

    async fn handle_dealing_exchange(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: dealing exchange (resharing: {resharing})");
        self.dealing_exchange(epoch_id, resharing)
            .await
            .map_err(|source| DkgError::DealingExchangeFailure { source })?;
        self.persist_state()
    }

    async fn handle_verification_key_submission(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: verification key submission (resharing: {resharing})");
        self.verification_key_submission(epoch_id, resharing)
            .await
            .map_err(|source| DkgError::VerificationKeySubmissionFailure { source })?;
        self.persist_state()
    }

    async fn handle_verification_key_validation(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: verification key validation (resharing: {resharing})");

        self.verification_key_validation(epoch_id)
            .await
            .map_err(|source| DkgError::VerificationKeyValidationFailure { source })?;
        self.persist_state()
    }

    async fn handle_verification_key_finalization(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DkgError> {
        debug!("DKG: verification key finalization (resharing: {resharing})");

        self.verification_key_finalization(epoch_id)
            .await
            .map_err(|source| DkgError::VerificationKeyFinalizationFailure { source })?;
        self.persist_state()
    }

    async fn handle_in_progress(&mut self, epoch_id: EpochId) -> Result<(), DkgError> {
        debug!("DKG: epoch in progress");

        let Ok(state) = self.state.in_progress_state(epoch_id) else {
            // we probably just started up the api while the DKG has already finished and we're waiting for new round to join
            debug!("the DKG has finished without our participation");
            return Ok(());
        };

        if !state.entered {
            info!("this is the first time this node is in the in progress state - going to clear state from the PREVIOUS epoch...");
            // if we finished dkg for epoch 123, we no longer care about anything from epoch 122
            // (but keep track of data from 123 for the future reference)
            self.state.clear_previous_epoch(epoch_id);

            // SAFETY: we just accessed this item in an immutable way, thus it MUST exist so the unwrap is fine
            self.state.in_progress_state_mut(epoch_id).unwrap().entered = true;
        }

        // so at this point we don't need to be polling the contract so often anymore, but we can't easily
        // adjust the existing interval.
        // however, what we can do is just wait here for a bit each iteration
        tokio::time::sleep(Duration::from_secs(120)).await;

        Ok(())
    }

    async fn check_if_can_advance_epoch_state(&self) -> Result<bool, DkgError> {
        debug!("checking if we can advance the epoch state");
        self.dkg_client
            .can_advance_epoch_state()
            .await
            .map_err(|source| DkgError::StateStatusQueryFailure { source })
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
        self.ensure_group_member().await?;

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
                self.handle_verification_key_validation(epoch.epoch_id, resharing)
                    .await?
            }
            EpochState::VerificationKeyFinalization { resharing } => {
                self.handle_verification_key_finalization(epoch.epoch_id, resharing)
                    .await?
            }
            // Just wait, in case we need to redo dkg at some point
            EpochState::InProgress => self.handle_in_progress(epoch.epoch_id).await?,
        };

        if self.check_if_can_advance_epoch_state().await? {
            // add a bit of variance so that all apis wouldn't attempt to trigger it at the same time
            let variance = self.rng.gen_range(0..=60);
            tokio::time::sleep(Duration::from_secs(variance)).await;

            // check if whether during our waiting somebody has already advanced the epoch
            if self.check_if_can_advance_epoch_state().await? {
                self.try_advance_dkg_state().await?
            }
        }

        Ok(())
    }

    fn reduced_tick_rate(&self, tick_duration: time::Duration) -> bool {
        // make sure to not trigger warnings if say the target rate is 10s, but our last tick took `9s999ms785µs321ns`
        // check for 95% of polling rate, so in that case if its below 9s500ms
        let target_nanos = self.polling_rate.as_nanos();
        let min = time::Duration::nanoseconds(((target_nanos * 95) / 100) as i64);
        tick_duration < min
    }

    pub(crate) async fn run(mut self, mut shutdown: TaskClient) {
        let mut interval = interval(self.polling_rate);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        // sometimes when the process is running behind, the ticker resolves multiple times in quick succession
        // so explicitly track those instances and make sure we don't overload the validator with contract calls
        let mut last_polled = OffsetDateTime::now_utc();
        let mut last_tick_duration = Default::default();

        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = interval.tick() => {
                    let now = OffsetDateTime::now_utc();
                    let tick_duration = now - last_polled;
                    last_polled = now;

                    if self.reduced_tick_rate(tick_duration) {
                        warn!("it seems the process is running behind. The current tick rate is lower than the polling rate. rate: {:?}, current tick: {}, previous tick: {}", self.polling_rate, tick_duration, last_tick_duration);
                        last_tick_duration = tick_duration;
                        continue
                    }
                    last_tick_duration = tick_duration;

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
    #[allow(dead_code)]
    pub(crate) fn default_test_mock(
        dkg_client: DkgClient,
        state: State,
    ) -> DkgController<rand_chacha::ChaCha20Rng> {
        DkgController {
            dkg_client,
            coconut_key_path: Default::default(),
            state,
            rng: crate::ecash::tests::fixtures::test_rng([1u8; 32]),
            polling_rate: Default::default(),
        }
    }

    pub(crate) fn test_mock(
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
