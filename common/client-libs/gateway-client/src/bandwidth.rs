// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
use crate::wasm_storage::{Storage, StorageError};
#[cfg(feature = "coconut")]
use coconut_interface::Base58;
#[cfg(feature = "coconut")]
#[cfg(not(target_arch = "wasm32"))]
use credential_storage::error::StorageError;
#[cfg(not(target_arch = "wasm32"))]
use credential_storage::storage::Storage;
#[cfg(feature = "coconut")]
use credentials::coconut::{
    bandwidth::prepare_for_spending, utils::obtain_aggregate_verification_key,
};
#[cfg(not(feature = "coconut"))]
use credentials::token::bandwidth::TokenCredential;
#[cfg(not(feature = "coconut"))]
use crypto::asymmetric::identity;
#[cfg(not(feature = "coconut"))]
use network_defaults::{
    eth_contract::ETH_ERC20_JSON_ABI, eth_contract::ETH_JSON_ABI, BANDWIDTH_VALUE,
    ETH_BURN_FUNCTION_NAME, ETH_CONTRACT_ADDRESS, ETH_ERC20_APPROVE_FUNCTION_NAME,
    ETH_ERC20_CONTRACT_ADDRESS, ETH_MIN_BLOCK_DEPTH, TOKENS_TO_BURN, UTOKENS_TO_BURN,
};
#[cfg(not(feature = "coconut"))]
use pemstore::traits::PemStorableKeyPair;
#[cfg(not(feature = "coconut"))]
use rand::rngs::OsRng;
#[cfg(not(feature = "coconut"))]
use secp256k1::SecretKey;
use std::str::FromStr;
#[cfg(not(feature = "coconut"))]
use web3::{
    contract::{Contract, Options},
    ethabi::Token,
    signing::{Key, SecretKeyRef},
    transports::Http,
    types::{Address, U256, U64},
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

#[cfg(not(feature = "coconut"))]
pub fn eth_erc20_contract(web3: Web3<Http>) -> Contract<Http> {
    Contract::from_json(
        web3.eth(),
        Address::from(ETH_ERC20_CONTRACT_ADDRESS),
        json::parse(ETH_ERC20_JSON_ABI)
            .expect("Invalid json abi")
            .dump()
            .as_bytes(),
    )
    .expect("Invalid json abi")
}

#[derive(Clone)]
pub struct BandwidthController<St: Storage> {
    storage: St,
    #[cfg(feature = "coconut")]
    validator_endpoints: Vec<url::Url>,
    #[cfg(not(feature = "coconut"))]
    contract: Contract<Http>,
    #[cfg(not(feature = "coconut"))]
    erc20_contract: Contract<Http>,
    #[cfg(not(feature = "coconut"))]
    eth_private_key: SecretKey,
}

impl<St> BandwidthController<St>
where
    St: Storage + Clone + 'static,
{
    #[cfg(feature = "coconut")]
    pub fn new(storage: St, validator_endpoints: Vec<url::Url>) -> Self {
        BandwidthController {
            storage,
            validator_endpoints,
        }
    }

    #[cfg(not(feature = "coconut"))]
    pub fn new(
        storage: St,
        eth_endpoint: String,
        eth_private_key: String,
    ) -> Result<Self, GatewayClientError> {
        // Fail early, on invalid url
        let transport =
            Http::new(&eth_endpoint).map_err(|_| GatewayClientError::InvalidURL(eth_endpoint))?;
        let web3 = web3::Web3::new(transport);
        // Fail early, on invalid abi
        let contract = eth_contract(web3.clone());
        let erc20_contract = eth_erc20_contract(web3);
        let eth_private_key = secp256k1::SecretKey::from_str(&eth_private_key)
            .map_err(|_| GatewayClientError::InvalidEthereumPrivateKey)?;

        Ok(BandwidthController {
            storage,
            contract,
            erc20_contract,
            eth_private_key,
        })
    }

    #[cfg(not(feature = "coconut"))]
    async fn backup_keypair(&self, keypair: &identity::KeyPair) -> Result<(), GatewayClientError> {
        self.storage
            .insert_erc20_credential(
                keypair.public_key().to_base58_string(),
                keypair.private_key().to_base58_string(),
            )
            .await?;

        Ok(())
    }

    #[cfg(not(feature = "coconut"))]
    async fn restore_keypair(&self) -> Result<identity::KeyPair, GatewayClientError> {
        let data = self.storage.get_next_erc20_credential().await?;
        let public_key = identity::PublicKey::from_base58_string(data.public_key).unwrap();
        let private_key = identity::PrivateKey::from_base58_string(data.private_key).unwrap();

        Ok(identity::KeyPair::from_keys(private_key, public_key))
    }

    #[cfg(not(feature = "coconut"))]
    async fn mark_keypair_as_spent(
        &self,
        keypair: &identity::KeyPair,
    ) -> Result<(), GatewayClientError> {
        self.storage
            .consume_erc20_credential(keypair.public_key().to_base58_string())
            .await?;

        Ok(())
    }

    #[cfg(feature = "coconut")]
    pub async fn prepare_coconut_credential(
        &self,
    ) -> Result<coconut_interface::Credential, GatewayClientError> {
        let verification_key = obtain_aggregate_verification_key(&self.validator_endpoints).await?;
        let bandwidth_credential = self.storage.get_next_coconut_credential().await?;
        let voucher_value = u64::from_str(&bandwidth_credential.voucher_value)
            .map_err(|_| StorageError::InconsistentData)?;
        let voucher_info = bandwidth_credential.voucher_info.clone();
        let serial_number =
            coconut_interface::Attribute::try_from_bs58(bandwidth_credential.serial_number)?;
        let binding_number =
            coconut_interface::Attribute::try_from_bs58(bandwidth_credential.binding_number)?;
        let signature =
            coconut_interface::Signature::try_from_bs58(bandwidth_credential.signature)?;

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        Ok(prepare_for_spending(
            voucher_value,
            voucher_info,
            serial_number,
            binding_number,
            &signature,
            &verification_key,
        )?)
    }

    #[cfg(not(feature = "coconut"))]
    pub async fn prepare_token_credential(
        &self,
        gateway_identity: identity::PublicKey,
        gateway_owner: String,
    ) -> Result<TokenCredential, GatewayClientError> {
        let kp = match self.restore_keypair().await {
            Ok(kp) => kp,
            Err(_) => {
                let mut rng = OsRng;
                let kp = identity::KeyPair::new(&mut rng);
                self.backup_keypair(&kp).await?;
                kp
            }
        };

        let verification_key = *kp.public_key();
        let signed_verification_key = kp.private_key().sign(&verification_key.to_bytes());
        self.buy_token_credential(verification_key, signed_verification_key, gateway_owner)
            .await?;

        self.mark_keypair_as_spent(&kp).await?;

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
        verification_key: identity::PublicKey,
        signed_verification_key: identity::Signature,
        gateway_owner: String,
    ) -> Result<(), GatewayClientError> {
        let confirmations = if cfg!(debug_assertions) {
            1
        } else {
            ETH_MIN_BLOCK_DEPTH
        };
        // 15 seconds per confirmation block + 10 seconds of network overhead + 20 seconds of wait for kill
        log::info!(
            "Waiting for Ethereum transaction. This should take about {} seconds",
            (confirmations + 1) * 15 + 30
        );
        let mut options = Options::default();
        let estimation = self
            .erc20_contract
            .estimate_gas(
                ETH_ERC20_APPROVE_FUNCTION_NAME,
                (
                    Token::Address(Address::from(ETH_CONTRACT_ADDRESS)),
                    Token::Uint(U256::from(UTOKENS_TO_BURN)),
                ),
                SecretKeyRef::from(&self.eth_private_key).address(),
                options.clone(),
            )
            .await?;
        options.gas = Some(estimation);
        log::info!("Calling ERC20 approve in 10 seconds with an estimated gas of {}. Kill the process if you want to abort", estimation);
        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        #[cfg(target_arch = "wasm32")]
        if let Err(err) = fluvio_wasm_timer::Delay::new(std::time::Duration::from_secs(10)).await {
            log::error!(
                "the timer has gone away while waiting for possible kill! - {}",
                err
            );
        }
        let recipt = self
            .erc20_contract
            .signed_call_with_confirmations(
                ETH_ERC20_APPROVE_FUNCTION_NAME,
                (
                    Token::Address(Address::from(ETH_CONTRACT_ADDRESS)),
                    Token::Uint(U256::from(UTOKENS_TO_BURN)),
                ),
                options,
                1, // One confirmation is enough, as we'll be consuming the approved token next anyway
                &self.eth_private_key,
            )
            .await?;
        if Some(U64::from(0u64)) == recipt.status {
            return Err(GatewayClientError::BurnTokenError(
                web3::Error::InvalidResponse(format!(
                    "Approve transaction status is 0 (failure): {:?}",
                    recipt.logs,
                )),
            ));
        } else {
            log::info!(
                "Approved {} tokens for bandwidth use on Ethereum",
                TOKENS_TO_BURN
            );
        }

        let mut options = Options::default();
        let estimation = self
            .contract
            .estimate_gas(
                ETH_BURN_FUNCTION_NAME,
                (
                    Token::Uint(U256::from(UTOKENS_TO_BURN)),
                    Token::Uint(U256::from(&verification_key.to_bytes())),
                    Token::Bytes(signed_verification_key.to_bytes().to_vec()),
                    Token::String(gateway_owner.clone()),
                ),
                SecretKeyRef::from(&self.eth_private_key).address(),
                options.clone(),
            )
            .await?;
        options.gas = Some(estimation);
        log::info!("Generating bandwidth on ETH contract in 10 seconds with an estimated gas of {}. \
         Kill the process if you want to abort. Keep in mind that if you abort now, you'll still have \
         some tokens approved for bandwidth spending from the previous action. \
         If you don't want that, you'll need to manually decreaseAllowance to revert the approval.", estimation);
        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        #[cfg(target_arch = "wasm32")]
        if let Err(err) = fluvio_wasm_timer::Delay::new(std::time::Duration::from_secs(10)).await {
            log::error!(
                "the timer has gone away while waiting for possible kill! - {}",
                err
            );
        }
        let recipt = self
            .contract
            .signed_call_with_confirmations(
                ETH_BURN_FUNCTION_NAME,
                (
                    Token::Uint(U256::from(UTOKENS_TO_BURN)),
                    Token::Uint(U256::from(&verification_key.to_bytes())),
                    Token::Bytes(signed_verification_key.to_bytes().to_vec()),
                    Token::String(gateway_owner),
                ),
                options,
                confirmations,
                &self.eth_private_key,
            )
            .await?;
        if Some(U64::from(0u64)) == recipt.status {
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
    fn parse_erc20_contract() {
        let transport =
            Http::new("https://rinkeby.infura.io/v3/00000000000000000000000000000000").unwrap();
        let web3 = web3::Web3::new(transport);
        // test no panic occurs
        eth_erc20_contract(web3);
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
