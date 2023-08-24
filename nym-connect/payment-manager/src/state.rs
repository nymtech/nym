// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::Client;
use crate::storage::Storage;

pub struct Config {
    denom: String,
}

impl Config {
    pub fn new(denom: String) -> Self {
        Config { denom }
    }

    pub fn denom(&self) -> &str {
        &self.denom
    }
}

pub struct State {
    pub(crate) storage: Storage,
    pub(crate) client: Client,
    pub(crate) config: Config,
}

impl State {
    pub(crate) async fn new(storage: Storage, client: Client, config: Config) -> Self {
        State {
            storage,
            client,
            config,
        }
    }
}
