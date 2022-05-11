// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use validator_client::nymd::SigningCosmWasmClient;

pub(crate) struct Publisher<C> {
    client: Client<C>,
}

impl<C> Publisher<C>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    pub(crate) async fn submit_dealing_commitment(&self) {
        // self.client.submit_dealing_commitment().await;
        //
    }
}
