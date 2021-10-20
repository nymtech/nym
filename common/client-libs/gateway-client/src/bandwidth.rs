// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(not(feature = "coconut"))]
use crate::error::GatewayClientError;
#[cfg(feature = "coconut")]
use credentials::coconut::{
    bandwidth::{obtain_signature, prepare_for_spending},
    utils::obtain_aggregate_verification_key,
};
use crypto::asymmetric::identity::PublicKey;
#[cfg(not(feature = "coconut"))]
use crypto::asymmetric::identity::Signature;
#[cfg(not(feature = "coconut"))]
use network_defaults::{
    eth_contract::ETH_JSON_ABI, BANDWIDTH_VALUE, ETH_CONTRACT_ADDRESS, TOKEN_BANDWIDTH_VALUE,
};
#[cfg(not(feature = "coconut"))]
use secp256k1::SecretKey;
#[cfg(not(feature = "coconut"))]
use std::str::FromStr;
#[cfg(not(feature = "coconut"))]
use web3::{
    contract::{Contract, Options},
    transports::Http,
    types::{Address, Bytes, U256, U64},
    Web3,
};

#[cfg(not(feature = "coconut"))]
pub fn eth_contract(web3: Web3<Http>) -> Contract<Http> {
    Contract::from_json(
        web3.eth(),
        Address::from(ETH_CONTRACT_ADDRESS),
        json::parse(ETH_JSON_ABI)
            .expect("Invalid json abi")
            .dump()
            .as_bytes(),
    )
    .expect("Invalid json abi")
}

#[derive(Clone)]
pub struct BandwidthController {
    #[cfg(feature = "coconut")]
    validator_endpoints: Vec<url::Url>,
    #[cfg(feature = "coconut")]
    identity: PublicKey,
    #[cfg(not(feature = "coconut"))]
    contract: Contract<Http>,
    #[cfg(not(feature = "coconut"))]
    eth_private_key: SecretKey,
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
    pub fn new(eth_endpoint: String, eth_private_key: String) -> Self {
        // Fail early, on invalid url
        let transport = Http::new(&eth_endpoint).expect("Invalid Ethereum URL");
        let web3 = web3::Web3::new(transport);
        // Fail early, on invalid abi
        let contract = eth_contract(web3);
        let eth_private_key =
            secp256k1::SecretKey::from_str(&eth_private_key).expect("Invalid Ethereum private key");

        BandwidthController {
            contract,
            eth_private_key,
        }
    }

    #[cfg(feature = "coconut")]
    pub async fn prepare_coconut_credential(&self) -> coconut_interface::Credential {
        let verification_key = obtain_aggregate_verification_key(&self.validator_endpoints)
            .await
            .expect("could not obtain aggregate verification key of validators");

        let bandwidth_credential =
            obtain_signature(&self.identity.to_bytes(), &self.validator_endpoints)
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
        let tokens_to_burn = BANDWIDTH_VALUE / TOKEN_BANDWIDTH_VALUE;
        // 0 means a transaction failure, 1 means success
        if Some(U64::from(0))
            == self
                .contract
                .signed_call_with_confirmations(
                    "burnTokenForAccessCode",
                    (
                        U256::from(tokens_to_burn),
                        U256::from(&verification_key.to_bytes()),
                        Bytes(signed_verification_key.to_bytes().to_vec()),
                    ),
                    Options::default(),
                    1,
                    &self.eth_private_key,
                )
                .await?
                .status
        {
            Err(GatewayClientError::BurnTokenError(
                web3::Error::InvalidResponse(String::from("Transaction status is 0 (failure)")),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "coconut"))]
    #[test]
    fn parse_contract() {
        let transport =
            Http::new("https://rinkeby.infura.io/v3/00000000000000000000000000000000").unwrap();
        let web3 = web3::Web3::new(transport);
        // test no panic occurs
        eth_contract(web3);
    }
}
