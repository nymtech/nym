// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, SystemTime};

use log::*;

use coconut_interface::{Credential, VerificationKey};
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
    nym_api_clients: Vec<CoconutApiClient>,
    nyxd_client: Client<SigningNyxdClient>,
    mix_denom_base: String,
    aggregated_verification_key: VerificationKey,
}

impl CoconutVerifier {
    pub fn new(
        api_clients: Vec<CoconutApiClient>,
        nyxd_client: Client<SigningNyxdClient>,
        mix_denom_base: String,
        aggregated_verification_key: VerificationKey,
    ) -> Result<Self, RequestHandlingError> {
        if api_clients.is_empty() {
            return Err(RequestHandlingError::NotEnoughNymAPIs {
                received: 0,
                needed: 1,
            });
        }
        Ok(CoconutVerifier {
            nym_api_clients: api_clients,
            nyxd_client,
            mix_denom_base,
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
        for client in self.nym_api_clients.iter() {
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
