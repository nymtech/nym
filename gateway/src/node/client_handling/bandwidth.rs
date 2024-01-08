// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_coconut_interface::Credential;

pub struct Bandwidth {
    value: u64,
}

impl Bandwidth {
    pub fn value(&self) -> u64 {
        self.value
    }
}

impl From<Credential> for Bandwidth {
    fn from(credential: Credential) -> Self {
        let token_value = credential.voucher_value();
        let bandwidth_bytes = token_value * nym_network_defaults::BYTES_PER_UTOKEN;
        Bandwidth {
            value: bandwidth_bytes,
        }
    }
}
