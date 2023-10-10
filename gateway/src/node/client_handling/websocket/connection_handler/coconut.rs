// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::authenticated::RequestHandlingError;
use log::*;
use nym_coconut_interface::Credential;
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::{
    nyxd::{
        contract_traits::{
            CoconutBandwidthSigningClient, DkgQueryClient, MultisigQueryClient,
            MultisigSigningClient,
        },
        cosmwasm_client::logs::{find_attribute, BANDWIDTH_PROPOSAL_ID},
        Coin, Fee,
    },
    CoconutApiClient, DirectSigningHttpRpcNyxdClient,
};
use std::time::{Duration, SystemTime};

const ONE_HOUR_SEC: u64 = 3600;
const MAX_FEEGRANT_UNYM: u128 = 10000;

pub(crate) struct CoconutVerifier {
    nyxd_client: DirectSigningHttpRpcNyxdClient,
    mix_denom_base: String,
}

impl CoconutVerifier {
    pub fn new(nyxd_client: DirectSigningHttpRpcNyxdClient) -> Self {
        let mix_denom_base = nyxd_client.current_chain_details().mix_denom.base.clone();

        CoconutVerifier {
            nyxd_client,
            mix_denom_base,
        }
    }

    pub async fn all_current_coconut_api_clients(
        &self,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        let epoch_id = self.nyxd_client.get_current_epoch().await?.epoch_id;
        self.all_coconut_api_clients(epoch_id).await
    }

    pub async fn all_coconut_api_clients(
        &self,
        epoch_id: u64,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        Ok(all_coconut_api_clients(&self.nyxd_client, epoch_id).await?)
    }

    pub async fn release_funds(
        &self,
        api_clients: Vec<CoconutApiClient>,
        credential: &Credential,
    ) -> Result<(), RequestHandlingError> {
        // Use a custom multiplier for revoke, as the default one (1.3)
        // isn't enough
        let revoke_fee = Some(Fee::Auto(Some(1.5)));

        let res = self
            .nyxd_client
            .spend_credential(
                Coin::new(
                    credential.voucher_value().into(),
                    self.mix_denom_base.clone(),
                ),
                credential.blinded_serial_number(),
                self.nyxd_client.address().to_string(),
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

        let proposal = self.nyxd_client.query_proposal(proposal_id).await?;
        if !credential.has_blinded_serial_number(&proposal.description)? {
            return Err(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal has different serial number"),
            });
        }

        let req = nym_api_requests::coconut::VerifyCredentialBody::new(
            credential.clone(),
            proposal_id,
            self.nyxd_client.address(),
        );
        for client in api_clients {
            self.nyxd_client
                .grant_allowance(
                    &client.cosmos_address,
                    vec![Coin::new(MAX_FEEGRANT_UNYM, self.mix_denom_base.clone())],
                    SystemTime::now().checked_add(Duration::from_secs(ONE_HOUR_SEC)),
                    // It would be nice to be able to filter deeper, but for now only the msg type filter is avaialable
                    vec![String::from("/cosmwasm.wasm.v1.MsgExecuteContract")],
                    "Create allowance to vote the release of funds".to_string(),
                    None,
                )
                .await?;
            let ret = client.api_client.verify_bandwidth_credential(&req).await;
            self.nyxd_client
                .revoke_allowance(
                    &client.cosmos_address,
                    "Cleanup the previous allowance for releasing funds".to_string(),
                    revoke_fee.clone(),
                )
                .await?;
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

        self.nyxd_client.execute_proposal(proposal_id, None).await?;

        Ok(())
    }
}
