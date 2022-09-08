// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use network_defaults::{
    setup_env,
    var_names::{API_VALIDATOR, MIXNET_CONTRACT_ADDRESS, NYMD_VALIDATOR, VESTING_CONTRACT_ADDRESS},
    NymNetworkDetails,
};
use validator_client::nymd::{self, AccountId, NymdClient, QueryNymdClient, SigningNymdClient};
pub use validator_client::validator_api::Client as ValidatorApiClient;

use crate::context::errors::ContextError;

pub mod errors;

pub type SigningClient = validator_client::nymd::NymdClient<SigningNymdClient>;
pub type QueryClient = validator_client::nymd::NymdClient<QueryNymdClient>;
pub type SigningClientWithValidatorAPI = validator_client::Client<SigningNymdClient>;
pub type QueryClientWithValidatorAPI = validator_client::Client<QueryNymdClient>;

#[derive(Debug)]
pub struct ClientArgs {
    pub config_env_file: Option<std::path::PathBuf>,
    pub nymd_url: Option<String>,
    pub validator_api_url: Option<String>,
    pub mnemonic: Option<bip39::Mnemonic>,
    pub mixnet_contract_address: Option<AccountId>,
    pub vesting_contract_address: Option<AccountId>,
}

pub fn get_network_details(args: &ClientArgs) -> Result<NymNetworkDetails, ContextError> {
    // let the network defaults crate handle setting up the env vars if the file arg is set, otherwise
    // it will default to what is already in env vars, falling back to mainnet
    setup_env(args.config_env_file.clone());

    // override the env vars with user supplied arguments, if set
    if let Some(nymd_url) = args.nymd_url.as_ref() {
        std::env::set_var(NYMD_VALIDATOR, nymd_url);
    }
    if let Some(validator_api_url) = args.validator_api_url.as_ref() {
        std::env::set_var(API_VALIDATOR, validator_api_url);
    }
    if let Some(mixnet_contract_address) = args.mixnet_contract_address.as_ref() {
        std::env::set_var(MIXNET_CONTRACT_ADDRESS, mixnet_contract_address.to_string());
    }
    if let Some(vesting_contract_address) = args.vesting_contract_address.as_ref() {
        std::env::set_var(
            VESTING_CONTRACT_ADDRESS,
            vesting_contract_address.to_string(),
        );
    }

    Ok(NymNetworkDetails::new_from_env())
}

pub fn create_signing_client(
    args: ClientArgs,
    network_details: &NymNetworkDetails,
) -> Result<SigningClient, ContextError> {
    let client_config = nymd::Config::try_from_nym_network_details(network_details)
        .expect("failed to construct valid validator client config with the provided network");

    // get mnemonic
    let mnemonic = match std::env::var("MNEMONIC") {
        Ok(value) => bip39::Mnemonic::parse(value)?,
        // env var MNEMONIC is not present, so try to fall back to arg --mnemonic ...
        Err(_) => match args.mnemonic {
            Some(value) => value,
            None => return Err(ContextError::MnemonicNotProvided), // no env var or arg provided
        },
    };

    let nymd_url = network_details
        .endpoints
        .first()
        .expect("network details are not defined")
        .nymd_url
        .as_str();

    match NymdClient::connect_with_mnemonic(client_config, nymd_url, mnemonic, None) {
        Ok(client) => Ok(client),
        Err(e) => Err(ContextError::NymdError(format!("{:?}", e))),
    }
}

pub fn create_query_client(
    network_details: &NymNetworkDetails,
) -> Result<QueryClient, ContextError> {
    let client_config = nymd::Config::try_from_nym_network_details(network_details)
        .expect("failed to construct valid validator client config with the provided network");

    let nymd_url = network_details
        .endpoints
        .first()
        .expect("network details are not defined")
        .nymd_url
        .as_str();

    match NymdClient::connect(client_config, nymd_url) {
        Ok(client) => Ok(client),
        Err(e) => Err(ContextError::NymdError(format!("{:?}", e))),
    }
}

pub fn create_signing_client_with_validator_api(
    args: ClientArgs,
    network_details: &NymNetworkDetails,
) -> Result<SigningClientWithValidatorAPI, ContextError> {
    let client_config = validator_client::Config::try_from_nym_network_details(network_details)
        .expect("failed to construct valid validator client config with the provided network");

    // get mnemonic
    let mnemonic = match std::env::var("MNEMONIC") {
        Ok(value) => bip39::Mnemonic::parse(value)?,
        // env var MNEMONIC is not present, so try to fall back to arg --mnemonic ...
        Err(_) => match args.mnemonic {
            Some(value) => value,
            None => return Err(ContextError::MnemonicNotProvided), // no env var or arg provided
        },
    };

    match validator_client::client::Client::new_signing(client_config, mnemonic) {
        Ok(client) => Ok(client),
        Err(e) => Err(ContextError::NymdError(format!("{:?}", e))),
    }
}

pub fn create_query_client_with_validator_api(
    network_details: &NymNetworkDetails,
) -> Result<QueryClientWithValidatorAPI, ContextError> {
    let client_config = validator_client::Config::try_from_nym_network_details(network_details)
        .expect("failed to construct valid validator client config with the provided network");

    match validator_client::client::Client::new_query(client_config) {
        Ok(client) => Ok(client),
        Err(e) => Err(ContextError::NymdError(format!("{:?}", e))),
    }
}
