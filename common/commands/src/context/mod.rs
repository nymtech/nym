// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::errors::ContextError;
use nym_network_defaults::{
    setup_env,
    var_names::{MIXNET_CONTRACT_ADDRESS, NYM_API, NYXD, VESTING_CONTRACT_ADDRESS},
    NymNetworkDetails,
};
pub use nym_validator_client::nym_api::Client as NymApiClient;
use nym_validator_client::nyxd::{self, AccountId, NyxdClient};
use nym_validator_client::{
    DirectSigningHttpRpcNyxdClient, DirectSigningHttpRpcValidatorClient, QueryHttpRpcNyxdClient,
    QueryHttpRpcValidatorClient,
};
use tap::prelude::*;

pub mod errors;

pub type SigningClient = DirectSigningHttpRpcNyxdClient;
pub type QueryClient = QueryHttpRpcNyxdClient;
pub type SigningClientWithNyxd = DirectSigningHttpRpcValidatorClient;
pub type QueryClientWithNyxd = QueryHttpRpcValidatorClient;

#[derive(Debug)]
pub struct ClientArgs {
    pub config_env_file: Option<std::path::PathBuf>,
    pub nyxd_url: Option<String>,
    pub nym_api_url: Option<String>,
    pub mnemonic: Option<bip39::Mnemonic>,
    pub mixnet_contract_address: Option<AccountId>,
    pub vesting_contract_address: Option<AccountId>,
}

pub fn get_network_details(args: &ClientArgs) -> Result<NymNetworkDetails, ContextError> {
    // let the network defaults crate handle setting up the env vars if the file arg is set, otherwise
    // it will default to what is already in env vars, falling back to mainnet
    setup_env(args.config_env_file.as_ref());

    // override the env vars with user supplied arguments, if set
    if let Some(nyxd_url) = args.nyxd_url.as_ref() {
        std::env::set_var(NYXD, nyxd_url);
    }
    if let Some(nym_api_url) = args.nym_api_url.as_ref() {
        std::env::set_var(NYM_API, nym_api_url);
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
    let client_config = nyxd::Config::try_from_nym_network_details(network_details)
        .tap_err(|err| log::error!("Failed to get client config - {err}"))?;

    // get mnemonic
    let mnemonic = match std::env::var("MNEMONIC") {
        Ok(value) => bip39::Mnemonic::parse(value)?,
        // env var MNEMONIC is not present, so try to fall back to arg --mnemonic ...
        Err(_) => match args.mnemonic {
            Some(value) => value,
            None => return Err(ContextError::MnemonicNotProvided), // no env var or arg provided
        },
    };

    let nyxd_url = network_details
        .endpoints
        .first()
        .expect("network details are not defined")
        .nyxd_url
        .as_str();

    match NyxdClient::connect_with_mnemonic(client_config, nyxd_url, mnemonic) {
        Ok(client) => Ok(client),
        Err(e) => Err(ContextError::NyxdError(format!("{e}"))),
    }
}

pub fn create_query_client(
    network_details: &NymNetworkDetails,
) -> Result<QueryClient, ContextError> {
    let client_config = nyxd::Config::try_from_nym_network_details(network_details)
        .tap_err(|err| log::error!("Failed to get client config - {err}"))?;

    let nyxd_url = network_details
        .endpoints
        .first()
        .expect("network details are not defined")
        .nyxd_url
        .as_str();

    match NyxdClient::connect(client_config, nyxd_url) {
        Ok(client) => Ok(client),
        Err(e) => Err(ContextError::NyxdError(format!("{e}"))),
    }
}

pub fn create_signing_client_with_nym_api(
    args: ClientArgs,
    network_details: &NymNetworkDetails,
) -> Result<SigningClientWithNyxd, ContextError> {
    let client_config = nym_validator_client::Config::try_from_nym_network_details(network_details)
        .tap_err(|err| log::error!("Failed to get client config - {err}"))?;

    // get mnemonic
    let mnemonic = match std::env::var("MNEMONIC") {
        Ok(value) => bip39::Mnemonic::parse(value)?,
        // env var MNEMONIC is not present, so try to fall back to arg --mnemonic ...
        Err(_) => match args.mnemonic {
            Some(value) => value,
            None => return Err(ContextError::MnemonicNotProvided), // no env var or arg provided
        },
    };

    match nym_validator_client::client::Client::new_signing(client_config, mnemonic) {
        Ok(client) => Ok(client),
        Err(e) => Err(ContextError::NyxdError(format!("{e}"))),
    }
}

pub fn create_query_client_with_nym_api(
    network_details: &NymNetworkDetails,
) -> Result<QueryClientWithNyxd, ContextError> {
    let client_config = nym_validator_client::Config::try_from_nym_network_details(network_details)
        .tap_err(|err| log::error!("Failed to get client config - {err}"))?;

    match nym_validator_client::client::Client::new_query(client_config) {
        Ok(client) => Ok(client),
        Err(e) => Err(ContextError::NyxdError(format!("{e}"))),
    }
}
