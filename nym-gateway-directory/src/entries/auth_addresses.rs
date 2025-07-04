// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt::Display;

pub use nym_sdk::mixnet::Recipient;

use crate::{Error, error::Result};

// optional, until we remove the wireguard feature flag
#[derive(Debug, Copy, Clone)]
pub struct AuthAddress(pub Option<Recipient>);

impl AuthAddress {
    pub(crate) fn try_from_base58_string(address: &str) -> Result<Self> {
        let recipient = Recipient::try_from_base58_string(address).map_err(|source| {
            Error::RecipientFormattingError {
                address: address.to_string(),
                source,
            }
        })?;
        Ok(AuthAddress(Some(recipient)))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AuthAddresses {
    entry_addr: AuthAddress,
    exit_addr: AuthAddress,
}

impl AuthAddresses {
    pub fn new(entry_addr: AuthAddress, exit_addr: AuthAddress) -> Self {
        AuthAddresses {
            entry_addr,
            exit_addr,
        }
    }

    pub fn entry(&self) -> AuthAddress {
        self.entry_addr
    }

    pub fn exit(&self) -> AuthAddress {
        self.exit_addr
    }
}

impl Display for AuthAddresses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "entry: {:?} exit: {:?}",
            self.entry_addr.0, self.exit_addr.0
        )
    }
}
