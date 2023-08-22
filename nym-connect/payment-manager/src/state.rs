// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::Storage;

pub struct State {
    pub(crate) storage: Storage,
}

impl State {
    pub(crate) async fn new(storage: Storage) -> Self {
        State { storage }
    }
}
