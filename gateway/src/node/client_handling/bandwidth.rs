// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "coconut")]
use std::convert::TryFrom;

#[cfg(feature = "coconut")]
use coconut_interface::Credential;
#[cfg(feature = "coconut")]
use credentials::error::Error;
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
impl TryFrom<Credential> for Bandwidth {
    type Error = Error;

    fn try_from(credential: Credential) -> Result<Self, Self::Error> {
        let value = credential.voucher_value()?;
        Ok(Self { value })
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
