// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TEMPORARY WORKAROUND:
// those features are expected as the below should only get activated whenever
// the corresponding features in tendermint-rpc are enabled transitively
#![allow(unexpected_cfgs)]

use crate::nyxd::cosmwasm_client::client_traits::SigningCosmWasmClient;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Config, GasPrice};
use crate::rpc::TendermintRpcClient;
use crate::signing::{
    signer::{NoSigner, OfflineSigner},
    AccountData,
};
use async_trait::async_trait;
use cosmrs::tx::{Raw, SignDoc};
use std::fmt::Debug;
use tendermint_rpc::{Error as TendermintRpcError, SimpleRequest};

pub use helpers::{ContractResponseData, ToContractResponseData};

#[cfg(feature = "http-client")]
use crate::http_client;
#[cfg(feature = "http-client")]
use cosmrs::rpc::{HttpClient, HttpClientUrl};

pub mod client_traits;
mod helpers;
pub mod logs;
pub mod module_traits;
pub mod types;

#[derive(Debug)]
pub(crate) struct SigningClientOptions {
    gas_price: GasPrice,
    simulated_gas_multiplier: f32,
}

impl<'a> From<&'a Config> for SigningClientOptions {
    fn from(value: &'a Config) -> Self {
        SigningClientOptions {
            gas_price: value.gas_price.clone(),
            simulated_gas_multiplier: value.simulated_gas_multiplier,
        }
    }
}

// convenience wrapper around query client to allow for optional signing
#[derive(Debug)]
pub(crate) struct MaybeSigningClient<C, S = NoSigner> {
    client: C,
    signer: S,
    opts: SigningClientOptions,
}

impl<C> MaybeSigningClient<C> {
    pub(crate) fn new(client: C, opts: SigningClientOptions) -> Self {
        MaybeSigningClient {
            client,
            signer: Default::default(),
            opts,
        }
    }
}

impl<C, S> MaybeSigningClient<C, S> {
    pub(crate) fn new_signing(client: C, signer: S, opts: SigningClientOptions) -> Self
    where
        S: OfflineSigner,
    {
        MaybeSigningClient {
            client,
            signer,
            opts,
        }
    }
}

#[cfg(feature = "http-client")]
impl<S> MaybeSigningClient<HttpClient, S> {
    pub(crate) fn change_endpoint<U>(&mut self, new_endpoint: U) -> Result<(), NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        self.client = http_client(new_endpoint)?;
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C, S> TendermintRpcClient for MaybeSigningClient<C, S>
where
    C: TendermintRpcClient + Send + Sync,
    S: Send + Sync,
{
    async fn perform<R>(&self, request: R) -> Result<R::Output, TendermintRpcError>
    where
        R: SimpleRequest,
    {
        self.client.perform(request).await
    }
}

impl<C, S> OfflineSigner for MaybeSigningClient<C, S>
where
    S: OfflineSigner,
{
    type Error = S::Error;

    fn get_accounts(&self) -> Result<Vec<AccountData>, Self::Error> {
        self.signer.get_accounts()
    }

    fn sign_direct_with_account(
        &self,
        signer: &AccountData,
        sign_doc: SignDoc,
    ) -> Result<Raw, Self::Error> {
        self.signer.sign_direct_with_account(signer, sign_doc)
    }
}

#[async_trait]
impl<C, S> SigningCosmWasmClient for MaybeSigningClient<C, S>
where
    C: TendermintRpcClient + Send + Sync,
    S: OfflineSigner + Send + Sync,
    NyxdError: From<S::Error>,
{
    fn gas_price(&self) -> &GasPrice {
        &self.opts.gas_price
    }

    fn simulated_gas_multiplier(&self) -> f32 {
        self.opts.simulated_gas_multiplier
    }
}

//
// #[cfg(feature = "http-client")]
// pub fn connect<U>(endpoint: U) -> Result<MaybeSigningClient<HttpClient>, NyxdError>
// where
//     U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
// {
//     Ok(HttpClient::new(endpoint)?)
// }
//
// #[cfg(all(feature = "signing", feature = "http-client"))]
// pub fn connect_with_signer<S, U: Clone>(
//     endpoint: U,
//     signer: S,
//     gas_price: crate::nyxd::GasPrice,
// ) -> Result<MaybeSigningClient<HttpClient, S>, NyxdError>
// where
//     U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
// {
//     signing_client::Client::connect_with_signer(endpoint, signer, gas_price)
// }
