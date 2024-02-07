// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use log::error;
use nym_credentials::coconut::bandwidth::CredentialType;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BandwidthError {}

pub struct Bandwidth {
    value: u64,
}

impl Bandwidth {
    pub const fn new(value: u64) -> Bandwidth {
        Bandwidth { value }
    }

    pub fn try_from_raw_value(value: &String, typ: CredentialType) -> Result<Self, BandwidthError> {
        // let bandwidth_value = match credential.data.typ {
        //     CredentialType::Voucher => {
        //         todo!()
        //     }
        //     CredentialType::FreePass => {
        //         error!("unimplemented handling of free pass credential");
        //         return Err(());
        //     }
        // };

        /*
          if bandwidth_value > i64::MAX as u64 {
            // note that this would have represented more than 1 exabyte,
            // which is like 125,000 worth of hard drives so I don't think we have
            // to worry about it for now...
            warn!("Somehow we received bandwidth value higher than 9223372036854775807. We don't really want to deal with this now");
            return Err(RequestHandlingError::UnsupportedBandwidthValue(
                bandwidth_value,
            ));
        }
         */

        todo!()
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}

// impl From<Credential> for Bandwidth {
//     fn from(credential: Credential) -> Self {
//         let token_value = credential.voucher_value();
//         let bandwidth_bytes = token_value * nym_network_defaults::BYTES_PER_UTOKEN;
//         Bandwidth {
//             value: bandwidth_bytes,
//         }
//     }
// }
