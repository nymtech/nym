// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::websocket::connection_handler::ecash::error::EcashTicketError;
use crate::node::Storage;
use crate::GatewayError;
use cosmwasm_std::{from_binary, CosmosMsg, WasmMsg};
use nym_credentials_interface::VerificationKeyAuth;
use nym_ecash_contract_common::msg::ExecuteMsg;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, MultisigQueryClient, NymContractsProvider,
};
use nym_validator_client::nyxd::cw3::ProposalResponse;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, EcashApiClient};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{error, trace, warn};

// state shared by different subtasks dealing with credentials
#[derive(Clone)]
pub(crate) struct SharedState<S> {
    pub(crate) nyxd_client: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,
    pub(crate) address: AccountId,
    pub(crate) epoch_data: Arc<RwLock<BTreeMap<EpochId, EpochState>>>,
    pub(crate) storage: S,
}

impl<S> SharedState<S>
where
    S: Storage + Clone,
{
    pub(crate) async fn new(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        storage: S,
    ) -> Result<Self, GatewayError> {
        let address = nyxd_client.address();

        if nyxd_client.dkg_contract_address().is_none() {
            error!("the DKG contract address is not available");
            return Err(EcashTicketError::UnavailableDkgContract.into());
        }

        let Ok(current_epoch) = nyxd_client.get_current_epoch().await else {
            error!("the specified DKG contract address is invalid - no coconut credentials will be redeemable");
            // if we require coconut credentials, we MUST have DKG contract available
            return Err(EcashTicketError::UnavailableDkgContract.into());
        };

        let this = SharedState {
            nyxd_client: Arc::new(RwLock::new(nyxd_client)),
            address,
            epoch_data: Arc::new(RwLock::new(BTreeMap::new())),
            storage,
        };

        // might as well obtain the data for the current epoch, if applicable
        if current_epoch.state.is_in_progress() {
            if let Err(err) = this.set_epoch_data(current_epoch.epoch_id).await {
                warn!("failed to set initial epoch data: {err}")
            }
        }

        Ok(this)
    }

    fn created_redemption_proposal(&self, proposal: &ProposalResponse) -> bool {
        let Some(msg) = proposal.msgs.first() else {
            return false;
        };
        let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = msg else {
            return false;
        };
        let Ok(ExecuteMsg::RedeemTickets { gw, .. }) = from_binary(msg) else {
            return false;
        };

        gw == self.address.as_ref()
    }

    /// retrieve all redemption proposals made by this gateway since, but excluding, the provided id
    pub(crate) async fn proposals_since(
        &self,
        proposal_id: u64,
    ) -> Result<Vec<ProposalResponse>, EcashTicketError> {
        Ok(self
            .start_query()
            .await
            .list_proposals(Some(proposal_id), None)
            .await
            .map_err(EcashTicketError::chain_query_failure)?
            .proposals
            .into_iter()
            .filter(|p| self.created_redemption_proposal(p))
            .collect())
    }

    /// retrieve all redemption proposals made by this gateway that are available on the last page of the query
    pub(crate) async fn last_proposal_page(
        &self,
    ) -> Result<Vec<ProposalResponse>, EcashTicketError> {
        Ok(self
            .start_query()
            .await
            .reverse_proposals(None, None)
            .await
            .map_err(EcashTicketError::chain_query_failure)?
            .proposals
            .into_iter()
            .filter(|p| self.created_redemption_proposal(p))
            .collect())
    }

    async fn set_epoch_data(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockWriteGuard<BTreeMap<EpochId, EpochState>>, EcashTicketError> {
        let Some(threshold) = self.threshold(epoch_id).await? else {
            return Err(EcashTicketError::DKGThresholdUnavailable { epoch_id });
        };

        // TODO: optimise: query nym-apis for aggregate key instead
        // (when the below code was originally written, that query didn't exist)
        let api_clients = self.query_api_clients(epoch_id).await?;

        if api_clients.len() < threshold as usize {
            return Err(EcashTicketError::NotEnoughNymAPIs {
                received: api_clients.len(),
                needed: threshold as usize,
            });
        }

        let aggregated_verification_key =
            nym_credentials::aggregate_verification_keys(&api_clients)?;

        let mut guard = self.epoch_data.write().await;

        self.storage
            .insert_epoch_signers(
                epoch_id as i64,
                api_clients.iter().map(|c| c.node_id as i64).collect(),
            )
            .await?;

        guard.insert(
            epoch_id,
            EpochState {
                api_clients,
                master_key: aggregated_verification_key,
                threshold,
            },
        );
        Ok(guard)
    }

    async fn query_api_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<EcashApiClient>, EcashTicketError> {
        Ok(all_ecash_api_clients(self.nyxd_client.read().await.deref(), epoch_id).await?)
    }

    pub(crate) async fn threshold(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<u64>, EcashTicketError> {
        self.nyxd_client
            .read()
            .await
            .get_epoch_threshold(epoch_id)
            .await
            .map_err(EcashTicketError::chain_query_failure)
    }

    pub(crate) async fn api_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<Vec<EcashApiClient>>, EcashTicketError> {
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
    ) -> Result<RwLockReadGuard<VerificationKeyAuth>, EcashTicketError> {
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

    pub(crate) async fn current_epoch_id(&self) -> Result<EpochId, EcashTicketError> {
        Ok(self
            .start_query()
            .await
            .get_current_epoch()
            .await
            .map_err(EcashTicketError::chain_query_failure)?
            .epoch_id)
    }
}

pub(crate) struct EpochState {
    // note: **CURRENTLY** api client addresses don't change during the epochs
    pub(crate) api_clients: Vec<EcashApiClient>,
    pub(crate) master_key: VerificationKeyAuth,

    #[allow(unused)]
    pub(crate) threshold: u64,
}
