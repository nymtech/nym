// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Result;
use crate::{CONTRACT_ADDRESS, MNEMONIC, NYMD_URL};
use bip39::Mnemonic;
use coconut_bandwidth_contract_common::deposit::DepositData;
use coconut_bandwidth_contract_common::msg::ExecuteMsg;
use network_defaults::DEFAULT_NETWORK;
use std::str::FromStr;
use url::Url;
use validator_client::nymd::{AccountId, Coin, Denom, NymdClient, SigningNymdClient};

pub(crate) struct Client {
    nymd_client: NymdClient<SigningNymdClient>,
    denom: Denom,
    contract_address: AccountId,
}

impl Client {
    pub fn new() -> Self {
        let nymd_url = Url::from_str(NYMD_URL).unwrap();
        let mnemonic = Mnemonic::from_str(MNEMONIC).unwrap();
        let nymd_client = NymdClient::connect_with_mnemonic(
            DEFAULT_NETWORK,
            nymd_url.as_ref(),
            None,
            None,
            None,
            mnemonic,
            None,
        )
        .unwrap();
        let denom = Denom::from_str(network_defaults::DENOM).unwrap();
        let contract_address = AccountId::from_str(CONTRACT_ADDRESS).unwrap();

        Client {
            nymd_client,
            denom,
            contract_address,
        }
    }

    pub async fn deposit(
        &self,
        amount: u64,
        info: &str,
        verification_key: String,
        encryption_key: String,
    ) -> Result<String> {
        let req = ExecuteMsg::DepositFunds {
            data: DepositData::new(info.to_string(), verification_key, encryption_key),
        };
        let funds = vec![Coin::new(amount as u128, self.denom.to_string())];
        Ok(self
            .nymd_client
            .execute(&self.contract_address, &req, Default::default(), "", funds)
            .await?
            .transaction_hash
            .to_string())
    }
}
