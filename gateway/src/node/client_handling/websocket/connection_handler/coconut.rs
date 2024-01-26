// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::authenticated::RequestHandlingError;
use log::*;
use nym_coconut_interface::{Credential, VerificationKey};
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::MultisigQueryClient;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{
    nyxd::{
        contract_traits::{CoconutBandwidthSigningClient, DkgQueryClient, MultisigSigningClient},
        cosmwasm_client::logs::{find_attribute, BANDWIDTH_PROPOSAL_ID},
        Coin,
    },
    CoconutApiClient, DirectSigningHttpRpcNyxdClient,
};
use std::collections::HashMap;
use std::ops::Deref;
use tokio::sync::{RwLock, RwLockReadGuard};

pub(crate) struct CoconutVerifier {
    address: AccountId,
    nyxd_client: RwLock<DirectSigningHttpRpcNyxdClient>,

    // **CURRENTLY** api client addresses don't change during the epochs
    api_clients: RwLock<HashMap<EpochId, Vec<CoconutApiClient>>>,

    // keys never change during epochs
    master_keys: RwLock<HashMap<EpochId, VerificationKey>>,
    mix_denom_base: String,
}

impl CoconutVerifier {
    pub async fn new(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
    ) -> Result<Self, RequestHandlingError> {
        let mix_denom_base = nyxd_client.current_chain_details().mix_denom.base.clone();
        let address = nyxd_client.address();

        let mut master_keys = HashMap::new();
        let mut api_clients = HashMap::new();

        // might as well obtain the key for the current epoch, if applicable
        let current_epoch = nyxd_client.get_current_epoch().await?;
        if current_epoch.state.is_in_progress() {
            // note: even though we're constructing clients here, we will NOT be making any network requests
            let epoch_api_clients =
                all_coconut_api_clients(&nyxd_client, current_epoch.epoch_id).await?;
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
                });
            }
            let aggregated_verification_key =
                nym_credentials::obtain_aggregate_verification_key(&epoch_api_clients).await?;

            api_clients.insert(current_epoch.epoch_id, epoch_api_clients);
            master_keys.insert(current_epoch.epoch_id, aggregated_verification_key);
        }

        Ok(CoconutVerifier {
            address,
            nyxd_client: RwLock::new(nyxd_client),
            api_clients: RwLock::new(api_clients),
            master_keys: RwLock::new(master_keys),
            mix_denom_base,
        })
    }

    pub async fn api_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<Vec<CoconutApiClient>>, RequestHandlingError> {
        let guard = self.api_clients.read().await;

        // the key was already in the map
        if let Ok(mapped) = RwLockReadGuard::try_map(guard, |clients| clients.get(&epoch_id)) {
            return Ok(mapped);
        }

        let api_clients = self.query_api_clients(epoch_id).await?;

        // EDGE CASE:
        // if this epoch is from the past, we can't query for its threshold
        // we can only hope that enough valid keys were submitted
        // the best we can do is check if we have at least a api
        if api_clients.is_empty() {
            return Err(RequestHandlingError::NotEnoughNymAPIs {
                received: 0,
                needed: 1,
            });
        }

        let mut guard = self.api_clients.write().await;
        guard.insert(epoch_id, api_clients);
        let guard = guard.downgrade();

        // SAFETY:
        // we just inserted the entry into the map while NEVER dropping the lock (only downgraded it)
        // so it MUST exist and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        Ok(RwLockReadGuard::map(guard, |clients| {
            clients.get(&epoch_id).unwrap()
        }))
    }

    pub async fn verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<VerificationKey>, RequestHandlingError> {
        let guard = self.master_keys.read().await;

        // the key was already in the map
        if let Ok(mapped) = RwLockReadGuard::try_map(guard, |keys| keys.get(&epoch_id)) {
            return Ok(mapped);
        }

        let api_clients = self.api_clients(epoch_id).await?;

        let mut guard = self.master_keys.write().await;

        let aggregated_verification_key =
            nym_credentials::obtain_aggregate_verification_key(&api_clients).await?;

        guard.insert(epoch_id, aggregated_verification_key);
        let guard = guard.downgrade();

        // SAFETY:
        // we just inserted the entry into the map while NEVER dropping the lock (only downgraded it)
        // so it MUST exist and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        Ok(RwLockReadGuard::map(guard, |keys| {
            keys.get(&epoch_id).unwrap()
        }))
    }

    pub async fn query_api_clients(
        &self,
        epoch_id: u64,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        Ok(all_coconut_api_clients(self.nyxd_client.read().await.deref(), epoch_id).await?)
    }

    pub async fn release_funds(
        &self,
        api_clients: &[CoconutApiClient],
        credential: &Credential,
    ) -> Result<(), RequestHandlingError> {
        let res = self
            .nyxd_client
            .write()
            .await
            .spend_credential(
                Coin::new(
                    credential.voucher_value().into(),
                    self.mix_denom_base.clone(),
                ),
                credential.blinded_serial_number(),
                self.address.to_string(),
                None,
            )
            .await?;
        let proposal_id = find_attribute(&res.logs, "wasm", BANDWIDTH_PROPOSAL_ID)
            .ok_or(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal id not found"),
            })?
            .value
            .parse::<u64>()
            .map_err(|_| RequestHandlingError::ProposalIdError {
                reason: String::from("proposal id could not be parsed to u64"),
            })?;

        let proposal = self
            .nyxd_client
            .read()
            .await
            .query_proposal(proposal_id)
            .await?;
        if !credential.has_blinded_serial_number(&proposal.description)? {
            return Err(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal has different serial number"),
            });
        }

        let req = nym_api_requests::coconut::VerifyCredentialBody::new(
            credential.clone(),
            proposal_id,
            self.address.clone(),
        );
        for client in api_clients {
            let ret = client.api_client.verify_bandwidth_credential(&req).await;
            match ret {
                Ok(res) => {
                    if !res.verification_result {
                        debug!("Validator {} didn't accept the credential. It will probably vote No on the spending proposal", client.api_client.nym_api.current_url());
                    }
                }
                Err(e) => {
                    warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", client.api_client.nym_api.current_url(), e);
                }
            }
        }

        self.nyxd_client
            .write()
            .await
            .execute_proposal(proposal_id, None)
            .await?;

        Ok(())
    }
}
