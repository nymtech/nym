// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::QueryRequest;
use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::cosmwasm_client::signing_client;
use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::fee_helpers::Operation;
pub use crate::nymd::gas_price::GasPrice;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use crate::ValidatorClientError;
use cosmos_sdk::rpc::{Error as TendermintRpcError, HttpClient, HttpClientUrl};
use cosmos_sdk::tx::Gas;
use cosmos_sdk::Coin as CosmosCoin;
use cosmos_sdk::{AccountId, Denom};
use cosmwasm_std::Coin;
use mixnet_contract::LayerDistribution;
use std::collections::HashMap;
use std::convert::TryInto;

pub mod cosmwasm_client;
pub(crate) mod fee_helpers;
pub mod gas_price;
pub mod wallet;

pub struct NymdClient<C> {
    client: C,
    contract_address: AccountId,
    client_address: Option<Vec<AccountId>>,
    gas_price: GasPrice,
    custom_gas_limits: HashMap<Operation, Gas>,
}

impl NymdClient<HttpClient> {
    pub fn connect<U>(
        endpoint: U,
        contract_address: AccountId,
    ) -> Result<NymdClient<HttpClient>, ValidatorClientError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        Ok(NymdClient {
            client: HttpClient::new(endpoint)?,
            contract_address,
            client_address: None,
            gas_price: Default::default(),
            custom_gas_limits: Default::default(),
        })
    }
}

impl NymdClient<signing_client::Client> {
    // maybe the wallet could be made into a generic, but for now, let's just have this one implementation
    pub fn connect_with_signer<U>(
        endpoint: U,
        contract_address: AccountId,
        signer: DirectSecp256k1HdWallet,
    ) -> Result<NymdClient<signing_client::Client>, ValidatorClientError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let client_address = signer
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();

        Ok(NymdClient {
            client: signing_client::Client::connect_with_signer(endpoint, signer)?,
            contract_address,
            client_address: Some(client_address),
            gas_price: Default::default(),
            custom_gas_limits: Default::default(),
        })
    }

    pub fn connect_with_mnemonic<U>(
        endpoint: U,
        contract_address: AccountId,
        mnemonic: bip39::Mnemonic,
    ) -> Result<NymdClient<signing_client::Client>, ValidatorClientError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(mnemonic)?;
        let client_address = wallet
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();

        Ok(NymdClient {
            client: signing_client::Client::connect_with_signer(endpoint, wallet)?,
            contract_address,
            client_address: Some(client_address),
            gas_price: Default::default(),
            custom_gas_limits: Default::default(),
        })
    }
}

impl<C> NymdClient<C> {
    pub fn set_gas_price(&mut self, gas_price: GasPrice) {
        self.gas_price = gas_price
    }

    pub fn set_custom_gas_limit(&mut self, operation: Operation, limit: Gas) {
        self.custom_gas_limits.insert(operation, limit);
    }

    pub fn address(&self) -> &AccountId
    where
        C: SigningCosmWasmClient,
    {
        // if this is a signing client (as required by the trait bound), it must have the address set
        &self.client_address.as_ref().unwrap()[0]
    }

    // now the question is as follows: will denom always be in the format of `u{prefix}`?
    pub fn denom(&self) -> Denom {
        format!("u{}", self.contract_address.prefix())
            .parse()
            .unwrap()
    }

    // just some example API (that will be expanded on in another PR) that those generics allow us to make:

    // this requires signing
    pub async fn bond_mixnode(
        &self,
        mixnode: MixNode,
        bond: Coin,
    ) -> Result<ExecuteResult, ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = Operation::BondMixnode.determine_fee(
            &self.gas_price,
            self.custom_gas_limits.get(&Operation::BondMixnode).cloned(),
        );

        let req = ExecuteMsg::BondMixnode { mix_node: mixnode };
        self.client
            .execute(
                self.address(),
                &self.contract_address,
                &req,
                fee,
                "Bonding mixnode from rust!",
                Some(vec![cosmwasm_coin_to_cosmos_coin(bond)]),
            )
            .await
    }

    // this is just a query
    pub async fn get_layer_distribution(&self) -> Result<LayerDistribution, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryRequest::LayerDistribution {};
        self.client
            .query_contract_smart(&self.contract_address, &request)
            .await
    }
}

// this will be extracted from the smart contract into the common crate later.
// right now it's just to show how it will look later
#[derive(serde::Serialize)]
pub struct MixNode {
    pub host: String,
    pub mix_port: u16,
    pub verloc_port: u16,
    pub http_api_port: u16,
    pub sphinx_key: String,
    pub identity_key: String,
    pub version: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    BondMixnode { mix_node: MixNode },
}

// this should go to helpers
fn cosmwasm_coin_to_cosmos_coin(coin: Coin) -> CosmosCoin {
    CosmosCoin {
        denom: coin.denom.parse().unwrap(),
        // this might be a bit iffy, cosmwasm coin stores value as u128, while cosmos does it as u64
        amount: (coin.amount.u128() as u64).into(),
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::coin;
//
//     #[tokio::test]
//     async fn test_bond() {
//         let validator = "http://127.0.0.1:26657";
//         let contract = "punk1uft3mmgha04vr23lx08fydpjym2003xuy9cnga"
//             .parse::<AccountId>()
//             .unwrap();
// (this is just mnemonic for validator running on my local machine, so feel free to 'steal' it)
//         let mnemonic = "claim border flee add vehicle crack romance assault fold wide flag year cousin false junk analyst parent eagle act visual tongue weasel basket impulse";
//
//         let client =
//             NymdClient::connect_with_mnemonic(validator, contract, mnemonic.parse().unwrap())
//                 .unwrap();
//
//         let mix = MixNode {
//             host: "1.1.1.1".to_string(),
//             mix_port: 1789,
//             verloc_port: 1790,
//             http_api_port: 8080,
//             sphinx_key: "sphinxkey".to_string(),
//             identity_key: "identitykey".to_string(),
//             version: "0.11.0".to_string(),
//         };
//
//         let result = client
//             .bond_mixnode(mix, coin(100_000000, client.denom().as_ref()))
//             .await
//             .unwrap();
//         println!("{:#?}", result)
//     }
//
//     #[tokio::test]
//     async fn test_get_layers() {
//         let validator = "https://testnet-milhon-validator1.nymtech.net";
//         let contract = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen"
//             .parse::<AccountId>()
//             .unwrap();
//
//         let client = NymdClient::connect(validator, contract).unwrap();
//         let layers = client.get_layer_distribution().await.unwrap();
//
//         println!("layers: {:#?}", layers);
//     }
// }
