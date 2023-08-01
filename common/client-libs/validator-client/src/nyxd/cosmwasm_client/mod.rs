// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::client_traits::{CosmWasmClient, SigningCosmWasmClient};
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Config, GasPrice, TendermintClient};
use async_trait::async_trait;
use cosmrs::AccountId;
use tendermint_rpc::{Error as TendermintRpcError, SimpleRequest};

#[cfg(feature = "http-client")]
use cosmrs::rpc::{HttpClient, HttpClientUrl};

use crate::signing::{
    signer::{NoSigner, OfflineSigner},
    tx_signer::TxSigner,
    AccountData,
};

pub mod client_traits;
mod helpers;
pub mod logs;
pub mod types;

#[derive(Debug)]
pub(crate) struct SigningClientOptions {
    gas_price: GasPrice,
}

impl<'a> From<&'a Config> for SigningClientOptions {
    fn from(value: &'a Config) -> Self {
        SigningClientOptions {
            gas_price: value.gas_price.clone(),
        }
    }
}

// convenience wrapper around query client to allow for optional signing
#[derive(Debug)]
pub(crate) struct MaybeSigningClient<C, S = NoSigner> {
    client: C,
    signer: S,
    opts: SigningClientOptions,
    derived_addresses: Option<Vec<AccountId>>,
}

impl<C> MaybeSigningClient<C> {
    pub(crate) fn new(client: C, opts: SigningClientOptions) -> Self {
        MaybeSigningClient {
            client,
            signer: Default::default(),
            opts,
            derived_addresses: None,
        }
    }
}

impl<C, S> MaybeSigningClient<C, S> {
    pub(crate) fn new_signing(
        client: C,
        signer: S,
        opts: SigningClientOptions,
    ) -> Result<Self, S::Error>
    where
        S: OfflineSigner,
    {
        let derived_addresses = signer
            .get_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();
        Ok(MaybeSigningClient {
            client,
            signer,
            opts,
            derived_addresses: Some(derived_addresses),
        })
    }

    pub(crate) fn derived_addresses(&self) -> &[AccountId] {
        // the unwrap is fine here as you can't construct a signing client without setting the addresses
        self.derived_addresses.as_ref().unwrap()
    }
}

#[cfg(feature = "http-client")]
impl<S> MaybeSigningClient<HttpClient, S> {
    pub(crate) fn change_endpoint<U>(&mut self, new_endpoint: U) -> Result<(), NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        self.client = HttpClient::new(new_endpoint)?;
        Ok(())
    }
}

#[async_trait]
impl<C, S> TendermintClient for MaybeSigningClient<C, S>
where
    C: TendermintClient + Send + Sync,
    S: Send + Sync,
{
    async fn perform<R>(&self, request: R) -> Result<R::Output, TendermintRpcError>
    where
        R: SimpleRequest,
    {
        self.client.perform(request).await
    }
}

#[async_trait]
impl<C, S> CosmWasmClient for MaybeSigningClient<C, S>
where
    C: CosmWasmClient + Send + Sync,
    S: Send + Sync,
{
}

impl<C, S> OfflineSigner for MaybeSigningClient<C, S>
where
    C: CosmWasmClient,
    S: OfflineSigner,
{
    type Error = S::Error;

    fn get_accounts(&self) -> Result<Vec<AccountData>, Self::Error> {
        self.signer.get_accounts()
    }
}

impl<C, S> TxSigner for MaybeSigningClient<C, S>
where
    C: CosmWasmClient,
    S: OfflineSigner,
{
}

#[async_trait]
impl<C, S> SigningCosmWasmClient for MaybeSigningClient<C, S>
where
    C: CosmWasmClient + Send + Sync,
    S: OfflineSigner + Send + Sync,
    NyxdError: From<S::Error>,
{
    fn gas_price(&self) -> &GasPrice {
        &self.opts.gas_price
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
