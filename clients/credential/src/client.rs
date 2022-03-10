// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bip39::Mnemonic;
use std::str::FromStr;
use url::Url;

use crate::{MNEMONIC, NYMD_URL};

use network_defaults::DEFAULT_NETWORK;
use validator_client::nymd::{NymdClient, SigningNymdClient};

pub(crate) struct Client {
    nymd_client: NymdClient<SigningNymdClient>,
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

        Client { nymd_client }
    }
}
