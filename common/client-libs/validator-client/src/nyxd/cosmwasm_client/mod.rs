// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "http-client")]
use crate::nyxd::error::NyxdError;
#[cfg(feature = "http-client")]
use cosmrs::rpc::{Error as TendermintRpcError, HttpClient, HttpClientUrl};
#[cfg(feature = "http-client")]
use std::convert::TryInto;

pub mod client;
mod helpers;
pub mod logs;
pub mod types;

#[cfg(feature = "signing")]
pub mod signing_client;

#[cfg(feature = "http-client")]
pub fn connect<U>(endpoint: U) -> Result<HttpClient, NyxdError>
where
    U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
{
    Ok(HttpClient::new(endpoint)?)
}

#[cfg(all(feature = "signing", feature = "http-client"))]
pub fn connect_with_signer<S, U: Clone>(
    endpoint: U,
    signer: S,
    gas_price: crate::nyxd::GasPrice,
) -> Result<signing_client::Client<S>, NyxdError>
where
    U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
{
    signing_client::Client::connect_with_signer(endpoint, signer, gas_price)
}
