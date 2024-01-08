// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::authenticated::RequestHandlingError;
use log::*;
use nym_coconut_interface::Credential;
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{
    nyxd::{
        contract_traits::{
            CoconutBandwidthSigningClient, DkgQueryClient, MultisigQueryClient,
            MultisigSigningClient,
        },
        cosmwasm_client::logs::{find_attribute, BANDWIDTH_PROPOSAL_ID},
        Coin,
    },
    CoconutApiClient, DirectSigningHttpRpcNyxdClient,
};
use std::ops::Deref;
use tokio::sync::RwLock;

pub(crate) struct CoconutVerifier {
    address: AccountId,
    nyxd_client: RwLock<DirectSigningHttpRpcNyxdClient>,
    mix_denom_base: String,
}

impl CoconutVerifier {
    pub async fn new(nyxd_client: DirectSigningHttpRpcNyxdClient) -> Self {
        let mix_denom_base = nyxd_client.current_chain_details().mix_denom.base.clone();
        let address = nyxd_client.address();

        CoconutVerifier {
            address,
            nyxd_client: RwLock::new(nyxd_client),
            mix_denom_base,
        }
    }

    pub async fn all_current_coconut_api_clients(
        &self,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        let epoch_id = self
            .nyxd_client
            .read()
            .await
            .get_current_epoch()
            .await?
            .epoch_id;
        self.all_coconut_api_clients(epoch_id).await
    }

    pub async fn all_coconut_api_clients(
        &self,
        epoch_id: u64,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        Ok(all_coconut_api_clients(self.nyxd_client.read().await.deref(), epoch_id).await?)
    }

    pub async fn release_funds(
        &self,
        api_clients: Vec<CoconutApiClient>,
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
