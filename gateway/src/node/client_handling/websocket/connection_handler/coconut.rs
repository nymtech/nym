// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, SystemTime};

use log::*;

use coconut_interface::{Credential, VerificationKey};
use network_defaults::MIX_DENOM;
use validator_client::{
    nymd::{traits::MultisigSigningClient, Coin, NymdClient, SigningNymdClient},
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
        let first_api_client = self
            .api_clients
            .get(0)
            .expect("This shouldn't happen, as we check for length in constructor");

        let first_api_cosmos_addr = first_api_client.get_cosmos_address().await?.addr;
        self.nymd_client
            .grant_allowance(
                &first_api_cosmos_addr,
                vec![Coin::new(MAX_FEEGRANT_UNYM, MIX_DENOM.base)],
                SystemTime::now().checked_add(Duration::from_secs(ONE_HOUR_SEC)),
                // It would be nice to be able to filter deeper, but for now only the msg type filter is avaialable
                vec![String::from("/cosmwasm.wasm.v1.MsgExecuteContract")],
                format!("Create allowance to propose the release of funds"),
                None,
            )
            .await?;

        let req = validator_api_requests::coconut::ProposeReleaseFundsRequestBody::new(
            credential.clone(),
            self.nymd_client.address().clone(),
        );
        let proposal_id = first_api_client
            .propose_release_funds(&req)
            .await?
            .proposal_id;

        let req = validator_api_requests::coconut::VerifyCredentialBody::new(
            credential.clone(),
            proposal_id,
            self.nymd_client.address().clone(),
        );
        for client in self.api_clients.iter().skip(1) {
            let api_cosmos_addr = client.get_cosmos_address().await?.addr;
            self.nymd_client
                .grant_allowance(
                    &api_cosmos_addr,
                    vec![Coin::new(MAX_FEEGRANT_UNYM, MIX_DENOM.base)],
                    SystemTime::now().checked_add(Duration::from_secs(ONE_HOUR_SEC)),
                    // It would be nice to be able to filter deeper, but for now only the msg type filter is avaialable
                    vec![String::from("/cosmwasm.wasm.v1.MsgExecuteContract")],
                    format!("Create allowance to vote the release of funds"),
                    None,
                )
                .await?;
            if !client
                .verify_bandwidth_credential(&req)
                .await?
                .verification_result
            {
                debug!("Validator {} didn't accept the credential. It will probably vote No on the spending proposal", client.validator_api.current_url());
            }
        }

        self.nymd_client.execute_proposal(proposal_id, None).await?;

        Ok(())
    }
}
