// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sphinx::addressing::clients::Recipient;

use crate::{Error, error::Result};

#[derive(Debug, Copy, Clone)]
pub struct AuthAddress(Recipient);

impl AuthAddress {
    pub(crate) fn try_from_base58_string(address: &str) -> Result<Self> {
        let recipient = Recipient::try_from_base58_string(address).map_err(|source| {
            Error::RecipientFormattingError {
                address: address.to_string(),
                source,
            }
        })?;
        Ok(AuthAddress(recipient))
    }
}

impl std::fmt::Display for AuthAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Recipient> for AuthAddress {
    fn from(recipient: Recipient) -> Self {
        Self(recipient)
    }
}

impl From<AuthAddress> for Recipient {
    fn from(auth_address: AuthAddress) -> Self {
        auth_address.0
    }
}
