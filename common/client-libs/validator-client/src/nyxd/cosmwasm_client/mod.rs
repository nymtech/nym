// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use crate::nyxd::wallet::DirectSecp256k1HdWallet;
use crate::nyxd::GasPrice;
use cosmrs::rpc::{Error as TendermintRpcError, HttpClient, HttpClientUrl};
use std::convert::TryInto;

pub mod client;
mod helpers;
pub mod logs;
pub mod signing_client;
pub mod types;

pub fn connect<U>(endpoint: U) -> Result<HttpClient, NyxdError>
where
    U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
{
    Ok(HttpClient::new(endpoint)?)
}

// maybe the wallet could be made into a generic, but for now, let's just have this one implementation
pub fn connect_with_signer<U: Clone>(
    endpoint: U,
    signer: DirectSecp256k1HdWallet,
    gas_price: GasPrice,
) -> Result<signing_client::Client, NyxdError>
where
    U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
{
    signing_client::Client::connect_with_signer(endpoint, signer, gas_price)
}
