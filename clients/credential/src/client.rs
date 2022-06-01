// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bip39::Mnemonic;
use std::str::FromStr;
use url::Url;

use crate::error::Result;
use crate::{MNEMONIC, NYMD_URL};

use network_defaults::{DEFAULT_NETWORK, MIX_DENOM, VOUCHER_INFO};
use validator_client::nymd::traits::CoconutBandwidthSigningClient;
use validator_client::nymd::{Coin, Fee, NymdClient, SigningNymdClient};

pub(crate) struct Client {
    nymd_client: NymdClient<SigningNymdClient>,
}

impl Client {
    pub fn new() -> Self {
        let nymd_url = Url::from_str(NYMD_URL).unwrap();
        let mnemonic = Mnemonic::from_str(MNEMONIC).unwrap();
        let nymd_client =
            NymdClient::connect_with_mnemonic(DEFAULT_NETWORK, nymd_url.as_ref(), mnemonic, None)
                .unwrap();

        Client { nymd_client }
    }

    pub async fn deposit(
        &self,
        amount: u64,
        verification_key: String,
        encryption_key: String,
        fee: Option<Fee>,
    ) -> Result<String> {
        let amount = Coin::new(amount as u128, MIX_DENOM.base.to_string());
        Ok(self
            .nymd_client
            .deposit(
                amount,
                String::from(VOUCHER_INFO),
                verification_key,
                encryption_key,
                fee,
            )
            .await?
            .transaction_hash
            .to_string())
    }
}
