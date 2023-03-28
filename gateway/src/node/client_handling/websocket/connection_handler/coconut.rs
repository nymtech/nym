// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_interface::Credential;
use coconut_interface::Credential;
use tracing::*;
use std::time::{Duration, SystemTime};
use validator_client::nyxd::traits::DkgQueryClient;
use validator_client::{
    nyxd::{
        cosmwasm_client::logs::{find_attribute, BANDWIDTH_PROPOSAL_ID},
        traits::{CoconutBandwidthSigningClient, MultisigQueryClient, MultisigSigningClient},
        Coin, Fee, SigningNyxdClient,
    },
    Client, CoconutApiClient,
};

use super::authenticated::RequestHandlingError;

const ONE_HOUR_SEC: u64 = 3600;
const MAX_FEEGRANT_UNYM: u128 = 10000;

pub(crate) struct CoconutVerifier {
    nyxd_client: Client<SigningNyxdClient>,
    mix_denom_base: String,
}

impl CoconutVerifier {
    pub fn new(nyxd_client: Client<SigningNyxdClient>) -> Self {
        let mix_denom_base = nyxd_client
            .nyxd
            .current_chain_details()
            .mix_denom
            .base
            .clone();

        CoconutVerifier {
            nyxd_client,
            mix_denom_base,
        }
    }

    pub async fn all_current_coconut_api_clients(
        &self,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        let epoch_id = self.nyxd_client.nyxd.get_current_epoch().await?.epoch_id;
        self.all_coconut_api_clients(epoch_id).await
    }

    pub async fn all_coconut_api_clients(
        &self,
        epoch_id: u64,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        Ok(CoconutApiClient::all_coconut_api_clients(&self.nyxd_client, epoch_id).await?)
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
            .nyxd
            .spend_credential(
                Coin::new(
                    credential.voucher_value().into(),
                    self.mix_denom_base.clone(),
                ),
                credential.blinded_serial_number(),
                self.nyxd_client.nyxd.address().to_string(),
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

        let proposal = self.nyxd_client.nyxd.get_proposal(proposal_id).await?;
        if !credential.has_blinded_serial_number(&proposal.description)? {
            return Err(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal has different serial number"),
            });
        }

        let req = nym_api_requests::coconut::VerifyCredentialBody::new(
            credential.clone(),
            proposal_id,
            self.nyxd_client.nyxd.address().clone(),
        );
        for client in api_clients {
            self.nyxd_client
                .nyxd
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
                .nyxd
                .revoke_allowance(
                    &client.cosmos_address,
                    "Cleanup the previous allowance for releasing funds".to_string(),
                    revoke_fee.clone(),
                )
                .await?;
            if !ret?.verification_result {
                debug!("Validator {} didn't accept the credential. It will probably vote No on the spending proposal", client.api_client.nym_api_client.current_url());
            }
        }

        self.nyxd_client
            .nyxd
            .execute_proposal(proposal_id, None)
            .await?;

        Ok(())
    }
}
