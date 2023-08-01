// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod client;
pub mod coconut;
#[cfg(feature = "http-client")]
pub mod connection_tester;
pub mod error;
pub mod nym_api;
pub mod nyxd;
pub mod signing;

pub use crate::error::ValidatorClientError;
pub use client::NymApiClient;
pub use client::{Client, CoconutApiClient, Config};
pub use nym_api_requests::*;

// some type aliasing

pub type ValidatorClient<C> = Client<C>;
pub type SigningValidatorClient<C, S> = Client<C, S>;

#[cfg(feature = "http-client")]
pub use cosmrs::rpc::HttpClient as HttpRpcClient;

#[cfg(feature = "http-client")]
use crate::signing::direct_wallet::DirectSecp256k1HdWallet;

#[cfg(feature = "http-client")]
pub type QueryHttpRpcValidatorClient = Client<HttpRpcClient>;
#[cfg(feature = "http-client")]
pub type QueryHttpRpcNyxdClient = nyxd::NyxdClient<HttpRpcClient>;

#[cfg(feature = "http-client")]
pub type DirectSigningHttpRpcValidatorClient = Client<HttpRpcClient, DirectSecp256k1HdWallet>;
#[cfg(feature = "http-client")]
pub type DirectSigningHttpRpcNyxdClient = nyxd::NyxdClient<HttpRpcClient, DirectSecp256k1HdWallet>;

// TODO: the same for reqwest client (once implemented)
