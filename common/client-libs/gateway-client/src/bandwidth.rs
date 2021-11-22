// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "coconut")]
use credentials::coconut::{
    bandwidth::{
        obtain_signature, prepare_for_spending, BandwidthVoucherAttributes, TOTAL_ATTRIBUTES,
    },
    utils::obtain_aggregate_verification_key,
};
#[cfg(not(feature = "coconut"))]
use credentials::token::bandwidth::TokenCredential;
#[cfg(not(feature = "coconut"))]
use crypto::asymmetric::identity;
use crypto::asymmetric::identity::PublicKey;
use network_defaults::BANDWIDTH_VALUE;
#[cfg(not(feature = "coconut"))]
use network_defaults::{
    eth_contract::ETH_JSON_ABI, ETH_BURN_FUNCTION_NAME, ETH_CONTRACT_ADDRESS, ETH_MIN_BLOCK_DEPTH,
    TOKENS_TO_BURN,
};
#[cfg(not(feature = "coconut"))]
use rand::rngs::OsRng;
#[cfg(not(feature = "coconut"))]
use secp256k1::SecretKey;
#[cfg(not(feature = "coconut"))]
use std::io::Write;
#[cfg(not(feature = "coconut"))]
use std::str::FromStr;
#[cfg(not(feature = "coconut"))]
use web3::{
    contract::{Contract, Options},
    transports::Http,
    types::{Address, Bytes, U256, U64},
    Web3,
};

use crate::error::GatewayClientError;

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
    #[cfg(not(feature = "coconut"))]
    backup_bandwidth_token_keys_dir: std::path::PathBuf,
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
    pub fn new(
        eth_endpoint: String,
        eth_private_key: String,
        backup_bandwidth_token_keys_dir: std::path::PathBuf,
    ) -> Result<Self, GatewayClientError> {
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
            backup_bandwidth_token_keys_dir,
        })
    }

    #[cfg(not(feature = "coconut"))]
    fn backup_keypair(&self, keypair: &identity::KeyPair) -> Result<(), GatewayClientError> {
        std::fs::create_dir_all(&self.backup_bandwidth_token_keys_dir)?;
        let file_path = self
            .backup_bandwidth_token_keys_dir
            .join(keypair.public_key().to_base58_string());
        let mut file = std::fs::File::create(file_path)?;
        file.write_all(&keypair.private_key().to_bytes())?;

        Ok(())
    }

    #[cfg(feature = "coconut")]
    pub async fn prepare_coconut_credential(
        &self,
    ) -> Result<coconut_interface::Credential, GatewayClientError> {
        let verification_key = obtain_aggregate_verification_key(&self.validator_endpoints).await?;
        let params = coconut_interface::Parameters::new(TOTAL_ATTRIBUTES).unwrap();

        // TODO: Decide what is the value and additional info associated with the bandwidth voucher
        let bandwidth_credential_attributes = BandwidthVoucherAttributes {
            serial_number: params.random_scalar(),
            binding_number: params.random_scalar(),
            voucher_value: coconut_interface::hash_to_scalar(BANDWIDTH_VALUE.to_be_bytes()),
            voucher_info: coconut_interface::hash_to_scalar(
                String::from("BandwidthVoucher").as_bytes(),
            ),
        };

        let bandwidth_credential = obtain_signature(
            &params,
            &bandwidth_credential_attributes,
            &self.validator_endpoints,
        )
        .await?;
        // the above would presumably be loaded from a file

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        Ok(prepare_for_spending(
            &self.identity.to_bytes(),
            &bandwidth_credential,
            &bandwidth_credential_attributes,
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
        self.backup_keypair(&kp)?;

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
        let confirmations = if cfg!(debug_assertions) {
            1
        } else {
            ETH_MIN_BLOCK_DEPTH
        };
        // 15 seconds per confirmation block + 10 seconds of network overhead
        log::info!(
            "Waiting for Ethereum transaction. This should take about {} seconds",
            confirmations * 15 + 10
        );
        let recipt = self
            .contract
            .signed_call_with_confirmations(
                ETH_BURN_FUNCTION_NAME,
                (
                    U256::from(TOKENS_TO_BURN),
                    U256::from(&verification_key.to_bytes()),
                    Bytes(signed_verification_key.to_bytes().to_vec()),
                ),
                Options::default(),
                confirmations,
                &self.eth_private_key,
            )
            .await?;
        if Some(U64::from(0)) == recipt.status {
            Err(GatewayClientError::BurnTokenError(
                web3::Error::InvalidResponse(format!(
                    "Transaction status is 0 (failure): {:?}",
                    recipt.logs,
                )),
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

#[cfg(not(feature = "coconut"))]
#[cfg(test)]
mod tests {
    use network_defaults::ETH_EVENT_NAME;

    use super::*;

    #[test]
    fn parse_contract() {
        let transport =
            Http::new("https://rinkeby.infura.io/v3/00000000000000000000000000000000").unwrap();
        let web3 = web3::Web3::new(transport);
        // test no panic occurs
        eth_contract(web3);
    }

    #[test]
    fn check_event_name_constant_against_abi() {
        let transport =
            Http::new("https://rinkeby.infura.io/v3/00000000000000000000000000000000").unwrap();
        let web3 = web3::Web3::new(transport);
        let contract = eth_contract(web3);
        assert!(contract.abi().event(ETH_EVENT_NAME).is_ok());
    }
}
