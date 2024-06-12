// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::RequestHandlingError;
use crate::GatewayError;
use log::{error, trace};
use nym_credentials_interface::VerificationKeyAuth;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, NymContractsProvider};
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{CoconutApiClient, DirectSigningHttpRpcNyxdClient};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// state shared by different subtasks dealing with credentials
#[derive(Clone)]
pub(crate) struct SharedState {
    pub(crate) nyxd_client: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,
    pub(crate) address: AccountId,
    pub(crate) epoch_data: Arc<RwLock<BTreeMap<EpochId, EpochState>>>,
}

impl SharedState {
    pub(crate) async fn new(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
    ) -> Result<Self, GatewayError> {
        let address = nyxd_client.address();

        if nyxd_client.dkg_contract_address().is_none() {
            error!("the DKG contract address is not available");
            return Err(RequestHandlingError::UnavailableDkgContract.into());
        }

        let Ok(current_epoch) = nyxd_client.get_current_epoch().await else {
            error!("the specified DKG contract address is invalid - no coconut credentials will be redeemable");
            // if we require coconut credentials, we MUST have DKG contract available
            return Err(RequestHandlingError::UnavailableDkgContract.into());
        };

        let mut epoch_data = BTreeMap::new();

        // might as well obtain the data for the current epoch, if applicable
        if current_epoch.state.is_in_progress() {
            // note: even though we're constructing clients here, we will NOT be making any network requests
            let epoch_api_clients = all_ecash_api_clients(&nyxd_client, current_epoch.epoch_id)
                .await
                .map_err(RequestHandlingError::from)?;
            let threshold = nyxd_client.get_current_epoch_threshold().await?;

            // SAFETY:
            // if epoch state is in the 'in progress' state, it means the threshold value MUST HAVE
            // been established. if it wasn't, there's an underlying issue with the DKG contract in which
            // case we shouldn't continue anyway because here be dragons
            #[allow(clippy::expect_used)]
            let threshold = threshold.expect("unavailable threshold value") as usize;
            if epoch_api_clients.len() < threshold {
                return Err(RequestHandlingError::NotEnoughNymAPIs {
                    received: epoch_api_clients.len(),
                    needed: threshold,
                }
                .into());
            }
            let aggregated_verification_key =
                nym_credentials::obtain_aggregate_verification_key(&epoch_api_clients)
                    .map_err(RequestHandlingError::from)?;

            epoch_data.insert(
                current_epoch.epoch_id,
                EpochState {
                    api_clients: epoch_api_clients,
                    master_key: aggregated_verification_key,
                },
            );
        }

        Ok(SharedState {
            nyxd_client: Arc::new(RwLock::new(nyxd_client)),
            address,
            epoch_data: Arc::new(RwLock::new(epoch_data)),
        })
    }

    async fn set_epoch_data(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockWriteGuard<BTreeMap<EpochId, EpochState>>, RequestHandlingError> {
        let api_clients = self.query_api_clients(epoch_id).await?;

        // EDGE CASE:
        // if this epoch is from the past, we can't query for its threshold
        // we can only hope that enough valid keys were submitted
        // the best we can do is check if we have at least a single api
        if api_clients.is_empty() {
            return Err(RequestHandlingError::NotEnoughNymAPIs {
                received: 0,
                needed: 1,
            });
        }

        let aggregated_verification_key =
            nym_credentials::obtain_aggregate_verification_key(&api_clients)?;

        let mut guard = self.epoch_data.write().await;
        guard.insert(
            epoch_id,
            EpochState {
                api_clients,
                master_key: aggregated_verification_key,
            },
        );
        Ok(guard)
    }

    async fn query_api_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        Ok(all_ecash_api_clients(self.nyxd_client.read().await.deref(), epoch_id).await?)
    }

    pub(crate) async fn api_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<Vec<CoconutApiClient>>, RequestHandlingError> {
        let guard = self.epoch_data.read().await;

        // the key was already in the map
        if let Ok(mapped) =
            RwLockReadGuard::try_map(guard, |data| data.get(&epoch_id).map(|d| &d.api_clients))
        {
            trace!("we already had cached api clients for epoch {epoch_id}");
            return Ok(mapped);
        }

        let write_guard = self.set_epoch_data(epoch_id).await?;
        let guard = write_guard.downgrade();

        // SAFETY:
        // we just inserted the entry into the map while NEVER dropping the lock (only downgraded it)
        // so it MUST exist and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        Ok(RwLockReadGuard::map(guard, |data| {
            data.get(&epoch_id).map(|d| &d.api_clients).unwrap()
        }))
    }

    pub(crate) async fn verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<VerificationKeyAuth>, RequestHandlingError> {
        let guard = self.epoch_data.read().await;

        // the key was already in the map
        if let Ok(mapped) =
            RwLockReadGuard::try_map(guard, |data| data.get(&epoch_id).map(|d| &d.master_key))
        {
            trace!("we already had cached api clients for epoch {epoch_id}");
            return Ok(mapped);
        }

        let write_guard = self.set_epoch_data(epoch_id).await?;
        let guard = write_guard.downgrade();

        // SAFETY:
        // we just inserted the entry into the map while NEVER dropping the lock (only downgraded it)
        // so it MUST exist and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        Ok(RwLockReadGuard::map(guard, |data| {
            data.get(&epoch_id).map(|d| &d.master_key).unwrap()
        }))
    }

    pub(crate) async fn start_tx(&self) -> RwLockWriteGuard<DirectSigningHttpRpcNyxdClient> {
        self.nyxd_client.write().await
    }

    pub(crate) async fn start_query(&self) -> RwLockReadGuard<DirectSigningHttpRpcNyxdClient> {
        self.nyxd_client.read().await
    }

    pub(crate) async fn current_epoch_id(&self) -> Result<EpochId, RequestHandlingError> {
        Ok(self.start_query().await.get_current_epoch().await?.epoch_id)
    }
}

pub(crate) struct EpochState {
    // note: **CURRENTLY** api client addresses don't change during the epochs
    pub(crate) api_clients: Vec<CoconutApiClient>,
    pub(crate) master_key: VerificationKeyAuth,
}
