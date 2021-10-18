// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(not(feature = "coconut"))]
use crate::error::GatewayClientError;
#[cfg(feature = "coconut")]
use credentials::{bandwidth::prepare_for_spending, obtain_aggregate_verification_key};
use crypto::asymmetric::identity::PublicKey;
#[cfg(not(feature = "coconut"))]
use crypto::asymmetric::identity::Signature;
#[cfg(not(feature = "coconut"))]
use network_defaults::{
    eth_contract::ETH_JSON_ABI, BANDWIDTH_VALUE, ETH_CONTRACT_ADDRESS, ETH_RPC_URL,
};
#[cfg(not(feature = "coconut"))]
use web3::{
    contract::{Contract, Options},
    transports::Http,
    types::{Address, Bytes, U256},
};

#[derive(Clone)]
pub struct BandwidthController {
    #[cfg(feature = "coconut")]
    validator_endpoints: Vec<url::Url>,
    #[cfg(feature = "coconut")]
    identity: PublicKey,
    #[cfg(not(feature = "coconut"))]
    contract: Contract<Http>,
}

impl BandwidthController {
    #[cfg(feature = "coconut")]
    pub fn new(validator_endpoints: Vec<url::Url>, identity: PublicKey) -> Self {
        BandwidthController {
            validator_endpoints,
            identity,
        }
    }

    #[cfg(not(feature = "coconut"))]
    pub fn new() -> Self {
        // Fail early, on invalid url
        let transport = Http::new(ETH_RPC_URL).expect("Invalid Ethereum URL");
        let web3 = web3::Web3::new(transport);
        // Fail early, on invalid abi
        let contract = Contract::from_json(
            web3.eth(),
            Address::from(ETH_CONTRACT_ADDRESS),
            json::parse(ETH_JSON_ABI)
                .expect("Invalid json abi")
                .dump()
                .as_bytes(),
        )
        .expect("Invalid json abi");

        BandwidthController { contract }
    }

    #[cfg(feature = "coconut")]
    pub async fn prepare_coconut_credential(&self) -> coconut_interface::Credential {
        let verification_key = obtain_aggregate_verification_key(&self.validator_endpoints)
            .await
            .expect("could not obtain aggregate verification key of validators");

        let bandwidth_credential = credentials::bandwidth::obtain_signature(
            &self.identity.to_bytes(),
            &self.validator_endpoints,
        )
        .await
        .expect("could not obtain bandwidth credential");
        // the above would presumably be loaded from a file

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        prepare_for_spending(
            &self.identity.to_bytes(),
            &bandwidth_credential,
            &verification_key,
        )
        .expect("could not prepare out bandwidth credential for spending")
    }

    #[cfg(not(feature = "coconut"))]
    pub async fn buy_token_credential(
        &self,
        verification_key: PublicKey,
        signed_verification_key: Signature,
    ) -> Result<(), GatewayClientError> {
        let key = secp256k1::key::ONE_KEY;
        self.contract
            .signed_call_with_confirmations(
                "burnTokenForAccessCode",
                (
                    U256::from(BANDWIDTH_VALUE),
                    U256::from(&verification_key.to_bytes()),
                    Bytes(signed_verification_key.to_bytes().to_vec()),
                ),
                Options::default(),
                1,
                &key,
            )
            .await
            .unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "coconut"))]
    #[test]
    fn parse_contract() {
        // test no panic occurs
        BandwidthController::new();
    }
}
