// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Result;
use crate::{MNEMONIC, NYMD_URL};
use bip39::Mnemonic;
use network_defaults::{DEFAULT_NETWORK, DENOM, VOUCHER_INFO};
use std::str::FromStr;
use url::Url;
use validator_client::nymd;
use validator_client::nymd::traits::CoconutBandwidthSigningClient;
use validator_client::nymd::{Coin, Fee, NymdClient, SigningNymdClient};

pub(crate) struct Client {
    nymd_client: NymdClient<SigningNymdClient>,
}

impl Client {
    pub fn new() -> Self {
        let nymd_url = Url::from_str(NYMD_URL).unwrap();
        let mnemonic = Mnemonic::from_str(MNEMONIC).unwrap();
        let config = nymd::Config::try_from_nym_network_details(&DEFAULT_NETWORK.details())
            .expect("failed to construct valid validator client config with the provided network");
        let nymd_client =
            NymdClient::connect_with_mnemonic(config, nymd_url.as_ref(), mnemonic, None).unwrap();

        Client { nymd_client }
    }

    pub async fn deposit(
        &self,
        amount: u64,
        verification_key: String,
        encryption_key: String,
        fee: Option<Fee>,
    ) -> Result<String> {
        let amount = Coin::new(amount as u128, DENOM.to_string());
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
