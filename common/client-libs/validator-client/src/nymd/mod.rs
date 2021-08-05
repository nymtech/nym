// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use crate::ValidatorClientError;
use cosmos_sdk::rpc::{Error as TendermintRpcError, HttpClientUrl};
use std::convert::TryInto;

pub mod cosmwasm_client;
pub mod wallet;

pub fn connect<U>(endpoint: U) -> Result<CosmWasmClient, ValidatorClientError>
where
    U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
{
    cosmwasm_client::connect(endpoint)
}

// maybe the wallet could be made into a generic, but for now, let's just have this one implementation
pub fn connect_with_signer<U>(
    endpoint: U,
    signer: DirectSecp256k1HdWallet,
) -> Result<SigningCosmWasmClient, ValidatorClientError>
where
    U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
{
    cosmwasm_client::connect_with_signer(endpoint, signer)
}
