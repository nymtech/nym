// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, SystemTime};

use log::*;

use coconut_interface::{Credential, VerificationKey};
use network_defaults::MIX_DENOM;
use validator_client::{
    nymd::{
        cosmwasm_client::logs::find_attribute,
        traits::{CoconutBandwidthSigningClient, MultisigQueryClient, MultisigSigningClient},
        Coin, Fee, NymdClient, SigningNymdClient,
    },
    ApiClient,
};

use super::authenticated::RequestHandlingError;

const ONE_HOUR_SEC: u64 = 3600;
const MAX_FEEGRANT_UNYM: u128 = 10000;

pub(crate) struct CoconutVerifier {
    api_clients: Vec<ApiClient>,
    nymd_client: NymdClient<SigningNymdClient>,
    aggregated_verification_key: VerificationKey,
}

impl CoconutVerifier {
    pub fn new(
        api_clients: Vec<ApiClient>,
        nymd_client: NymdClient<SigningNymdClient>,
        aggregated_verification_key: VerificationKey,
    ) -> Result<Self, RequestHandlingError> {
        if api_clients.is_empty() {
            return Err(RequestHandlingError::NotEnoughValidatorAPIs {
                received: 0,
                needed: 1,
            });
        }
        Ok(CoconutVerifier {
            api_clients,
            nymd_client,
            aggregated_verification_key,
        })
    }

    pub fn aggregated_verification_key(&self) -> &VerificationKey {
        &self.aggregated_verification_key
    }

    pub async fn release_funds(&self, credential: &Credential) -> Result<(), RequestHandlingError> {
        // Use a custom multiplier for revoke, as the default one (1.3)
        // isn't enough
        let revoke_fee = Some(Fee::Auto(Some(1.5)));

        let res = self
            .nymd_client
            .spend_credential(
                Coin::new(credential.voucher_value().into(), MIX_DENOM.base),
                credential.blinded_serial_number(),
                self.nymd_client.address().to_string(),
                None,
            )
            .await?;
        let proposal_id = find_attribute(&res.logs, "wasm", "proposal_id")
            .ok_or(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal id not found"),
            })?
            .value
            .parse::<u64>()
            .or(Err(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal id could not be parsed to u64"),
            }))?;

        let proposal = self.nymd_client.get_proposal(proposal_id).await?;
        if !credential.has_blinded_serial_number(&proposal.description)? {
            return Err(RequestHandlingError::ProposalIdError {
                reason: String::from("proposal has different serial number"),
            });
        }

        let req = validator_api_requests::coconut::VerifyCredentialBody::new(
            credential.clone(),
            proposal_id,
            self.nymd_client.address().clone(),
        );
        for client in self.api_clients.iter() {
            let api_cosmos_addr = client.get_cosmos_address().await?.addr;
            self.nymd_client
                .grant_allowance(
                    &api_cosmos_addr,
                    vec![Coin::new(MAX_FEEGRANT_UNYM, MIX_DENOM.base)],
                    SystemTime::now().checked_add(Duration::from_secs(ONE_HOUR_SEC)),
                    // It would be nice to be able to filter deeper, but for now only the msg type filter is avaialable
                    vec![String::from("/cosmwasm.wasm.v1.MsgExecuteContract")],
                    "Create allowance to vote the release of funds".to_string(),
                    None,
                )
                .await?;
            let ret = client.verify_bandwidth_credential(&req).await;
            self.nymd_client
                .revoke_allowance(
                    &api_cosmos_addr,
                    "Cleanup the previous allowance for releasing funds".to_string(),
                    revoke_fee.clone(),
                )
                .await?;
            if !ret?.verification_result {
                debug!("Validator {} didn't accept the credential. It will probably vote No on the spending proposal", client.validator_api.current_url());
            }
        }

        self.nymd_client.execute_proposal(proposal_id, None).await?;

        Ok(())
    }
}
