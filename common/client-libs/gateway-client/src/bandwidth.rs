// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayClientError;
#[cfg(feature = "coconut")]
use credentials::coconut::{
    bandwidth::{obtain_signature, prepare_for_spending},
    utils::obtain_aggregate_verification_key,
};
#[cfg(not(feature = "coconut"))]
use credentials::token::bandwidth::TokenCredential;
#[cfg(not(feature = "coconut"))]
use crypto::asymmetric::identity;
use crypto::asymmetric::identity::PublicKey;
#[cfg(not(feature = "coconut"))]
use network_defaults::{
    eth_contract::ETH_JSON_ABI, BANDWIDTH_VALUE, ETH_BURN_FUNCTION_NAME, ETH_CONTRACT_ADDRESS,
    TOKENS_TO_BURN,
};
#[cfg(not(feature = "coconut"))]
use rand::rngs::OsRng;
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
    pub fn new(eth_endpoint: String, eth_private_key: String) -> Result<Self, GatewayClientError> {
        // Fail early, on invalid url
        let transport =
            Http::new(&eth_endpoint).map_err(|_| GatewayClientError::InvalidURL(eth_endpoint))?;
        let web3 = web3::Web3::new(transport);
        // Fail early, on invalid abi
        let contract = eth_contract(web3);
        let eth_private_key = secp256k1::SecretKey::from_str(&eth_private_key)
            .map_err(|_| GatewayClientError::InvalidEthereumPrivateKey)?;

        Ok(BandwidthController {
            contract,
            eth_private_key,
        })
    }

    #[cfg(feature = "coconut")]
    pub async fn prepare_coconut_credential(
        &self,
    ) -> Result<coconut_interface::Credential, GatewayClientError> {
        let verification_key = obtain_aggregate_verification_key(&self.validator_endpoints).await?;

        let bandwidth_credential =
            obtain_signature(&self.identity.to_bytes(), &self.validator_endpoints).await?;
        // the above would presumably be loaded from a file

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        Ok(prepare_for_spending(
            &self.identity.to_bytes(),
            &bandwidth_credential,
            &verification_key,
        )?)
    }

    #[cfg(not(feature = "coconut"))]
    pub async fn prepare_token_credential(
        &self,
        gateway_identity: PublicKey,
    ) -> Result<TokenCredential, GatewayClientError> {
        let mut rng = OsRng;

        let kp = identity::KeyPair::new(&mut rng);

        let verification_key = *kp.public_key();
        let signed_verification_key = kp.private_key().sign(&verification_key.to_bytes());
        self.buy_token_credential(verification_key, signed_verification_key)
            .await?;

        let message: Vec<u8> = verification_key
            .to_bytes()
            .iter()
            .chain(gateway_identity.to_bytes().iter())
            .copied()
            .collect();
        let signature = kp.private_key().sign(&message);

        Ok(TokenCredential::new(
            verification_key,
            gateway_identity,
            BANDWIDTH_VALUE,
            signature,
        ))
    }

    #[cfg(not(feature = "coconut"))]
    pub async fn buy_token_credential(
        &self,
        verification_key: PublicKey,
        signed_verification_key: identity::Signature,
    ) -> Result<(), GatewayClientError> {
        // 0 means a transaction failure, 1 means success
        if Some(U64::from(0))
            == self
                .contract
                .signed_call_with_confirmations(
                    ETH_BURN_FUNCTION_NAME,
                    (
                        U256::from(TOKENS_TO_BURN),
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
            log::info!(
                "Bought bandwidth on Ethereum: {} MB",
                BANDWIDTH_VALUE / 1024 / 1024
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_defaults::ETH_EVENT_NAME;

    #[cfg(not(feature = "coconut"))]
    #[test]
    fn parse_contract() {
        let transport =
            Http::new("https://rinkeby.infura.io/v3/00000000000000000000000000000000").unwrap();
        let web3 = web3::Web3::new(transport);
        // test no panic occurs
        eth_contract(web3);
    }

    #[cfg(not(feature = "coconut"))]
    #[test]
    fn check_event_name_constant_against_abi() {
        let transport =
            Http::new("https://rinkeby.infura.io/v3/00000000000000000000000000000000").unwrap();
        let web3 = web3::Web3::new(transport);
        let contract = eth_contract(web3);
        assert!(contract.abi().event(ETH_EVENT_NAME).is_ok());
    }
}
