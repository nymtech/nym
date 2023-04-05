// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Result;
use bip39::Mnemonic;
use nym_network_defaults::{NymNetworkDetails, VOUCHER_INFO};
use std::str::FromStr;
use url::Url;
use nym_validator_client::nyxd;
use nym_validator_client::nyxd::traits::CoconutBandwidthSigningClient;
use nym_validator_client::nyxd::{Coin, DirectSigningNyxdClient, Fee, NyxdClient};

pub(crate) struct Client {
    nyxd_client: NyxdClient<DirectSigningNyxdClient>,
    mix_denom_base: String,
}

impl Client {
    pub fn new(nyxd_url: &str, mnemonic: &str) -> Self {
        let nyxd_url = Url::from_str(nyxd_url).unwrap();
        let mnemonic = Mnemonic::from_str(mnemonic).unwrap();
        let network_details = NymNetworkDetails::new_from_env();
        let config = nyxd::Config::try_from_nym_network_details(&network_details)
            .expect("failed to construct valid validator client config with the provided network");
        let nyxd_client =
            NyxdClient::connect_with_mnemonic(config, nyxd_url.as_ref(), mnemonic, None).unwrap();

        Client {
            nyxd_client,
            mix_denom_base: network_details.chain_details.mix_denom.base,
        }
    }

    pub async fn deposit(
        &self,
        amount: u64,
        verification_key: String,
        encryption_key: String,
        fee: Option<Fee>,
    ) -> Result<String> {
        let amount = Coin::new(amount as u128, self.mix_denom_base.clone());
        Ok(self
            .nyxd_client
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
