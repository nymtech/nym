// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use nym_sphinx_types::PrivateKey;

#[derive(Clone)]
pub struct SphinxPacketProcessor {
    /// Private sphinx key of this node required to unwrap received sphinx packet.
    sphinx_key: Arc<PrivateKey>,
}

impl SphinxPacketProcessor {
    /// Creates new instance of `CachedPacketProcessor`
    pub fn new(sphinx_key: PrivateKey) -> Self {
        SphinxPacketProcessor {
            sphinx_key: Arc::new(sphinx_key),
        }
    }

    pub fn sphinx_key(&self) -> &PrivateKey {
        &self.sphinx_key
    }
}
