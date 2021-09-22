// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;

use coconut_interface::Credential;
use credentials::error::Error;

const BANDWIDTH_INDEX: usize = 0;

pub struct Bandwidth {
    value: u64,
}

impl Bandwidth {
    pub fn value(&self) -> u64 {
        self.value
    }
}

impl TryFrom<Credential> for Bandwidth {
    type Error = Error;

    fn try_from(credential: Credential) -> Result<Self, Self::Error> {
        match credential.public_attributes().get(BANDWIDTH_INDEX) {
            None => Err(Error::NotEnoughPublicAttributes),
            Some(attr) => match <[u8; 8]>::try_from(attr.as_slice()) {
                Ok(bandwidth_bytes) => {
                    let value = u64::from_be_bytes(bandwidth_bytes);
                    Ok(Self { value })
                }
                Err(_) => Err(Error::InvalidBandwidthSize),
            },
        }
    }
}
