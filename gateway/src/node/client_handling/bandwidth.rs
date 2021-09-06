// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::RwLock;

use coconut_interface::Credential;
use credentials::error::Error;
use nymsphinx::DestinationAddressBytes;

pub type BandwidthDatabase = Arc<RwLock<HashMap<DestinationAddressBytes, AtomicU64>>>;

const BANDWIDTH_INDEX: usize = 0;

pub struct Bandwidth {
    value: u64,
}

impl Bandwidth {
    pub fn value(&self) -> u64 {
        self.value
    }

    pub async fn consume_bandwidth(
        bandwidths: &BandwidthDatabase,
        remote_address: &DestinationAddressBytes,
        consumed: u64,
    ) -> bool {
        if let Some(bandwidth) = bandwidths.write().await.get_mut(remote_address) {
            let bandwidth_mut = bandwidth.get_mut();
            if *bandwidth_mut >= consumed {
                *bandwidth_mut -= consumed;
                return true;
            }
        }
        false
    }

    pub async fn increase_bandwidth(
        bandwidths: &BandwidthDatabase,
        remote_address: &DestinationAddressBytes,
        increase: u64,
    ) -> Result<(), Error> {
        let mut db = bandwidths.write().await;
        if let Some(bandwidth) = db.get_mut(remote_address) {
            let bandwidth_mut = bandwidth.get_mut();
            if let Some(new_bandwidth) = bandwidth_mut.checked_add(increase) {
                *bandwidth_mut = new_bandwidth;
            } else {
                return Err(Error::BandwidthOverflow);
            }
        } else {
            db.insert(*remote_address, AtomicU64::new(increase));
        }
        Ok(())
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
