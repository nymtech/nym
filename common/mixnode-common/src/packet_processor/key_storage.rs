// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use nym_sphinx_types::{
    SharedSecret, RoutingKeys
};


#[derive(Clone, Debug)]
pub struct KeyStorage(Arc<Mutex<HashMap<SharedSecret, (RoutingKeys, Option<SharedSecret>)>>>);

impl KeyStorage {
    pub fn new() -> Self {
        KeyStorage(Arc::new(Mutex::new(
            HashMap::new()
        )))
    }


    pub fn lookup(&self, key : SharedSecret) -> Option<(RoutingKeys, Option<SharedSecret>)> {
        match self.0.lock() {
            Ok(map) => map.get(&key).cloned(),
            Err(_) => None,
        }
    }

    pub fn store(&self, key : SharedSecret, routing_key : RoutingKeys, blinded_shared_secret : Option<SharedSecret>) {
        match self.0.lock() {
            Ok(mut map) => map.insert(key, (routing_key, blinded_shared_secret)),
            Err(_) => return,
        };
    }

}


