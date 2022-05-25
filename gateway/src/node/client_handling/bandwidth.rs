// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "coconut")]
use coconut_interface::Credential;
#[cfg(not(feature = "coconut"))]
use credentials::token::bandwidth::TokenCredential;

pub struct Bandwidth {
    value: u64,
}

impl Bandwidth {
    pub fn value(&self) -> u64 {
        self.value
    }
}

#[cfg(feature = "coconut")]
impl From<Credential> for Bandwidth {
    fn from(credential: Credential) -> Self {
        Bandwidth {
            value: credential.voucher_value(),
        }
    }
}

#[cfg(not(feature = "coconut"))]
impl From<TokenCredential> for Bandwidth {
    fn from(credential: TokenCredential) -> Self {
        Bandwidth {
            value: credential.bandwidth(),
        }
    }
}
