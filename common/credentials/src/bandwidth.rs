// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// for time being assume the bandwidth credential consists of public identity of the requester
// and private (though known... just go along with it) infinite bandwidth value
// right now this has no double-spending protection, spender binding, etc
// it's the simplest possible case

use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

use crate::error::Error;
use crate::utils::{obtain_aggregate_signature, prepare_credential_for_spending};
use coconut_interface::{hash_to_scalar, Credential, Parameters, Signature, VerificationKey};
use nymsphinx::DestinationAddressBytes;

pub type BandwidthDatabase = Arc<RwLock<HashMap<DestinationAddressBytes, AtomicU64>>>;

const BANDWIDTH_VALUE: u64 = 1024 * 1024; // 1 MB
const BANDWIDTH_INDEX: usize = 0;

pub const PUBLIC_ATTRIBUTES: u32 = 1;
pub const PRIVATE_ATTRIBUTES: u32 = 1;
pub const TOTAL_ATTRIBUTES: u32 = PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES;

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

// TODO: this definitely has to be moved somewhere else. It's just a temporary solution
pub async fn obtain_signature(raw_identity: &[u8], validators: &[Url]) -> Result<Signature, Error> {
    let public_attributes = vec![hash_to_scalar(BANDWIDTH_VALUE.to_be_bytes())];
    let private_attributes = vec![hash_to_scalar(raw_identity)];

    let params = Parameters::new(TOTAL_ATTRIBUTES)?;

    obtain_aggregate_signature(&params, &public_attributes, &private_attributes, validators).await
}

pub fn prepare_for_spending(
    raw_identity: &[u8],
    signature: &Signature,
    verification_key: &VerificationKey,
) -> Result<Credential, Error> {
    let public_attributes = vec![BANDWIDTH_VALUE.to_be_bytes().to_vec()];
    let private_attributes = vec![raw_identity.to_vec()];

    let params = Parameters::new(TOTAL_ATTRIBUTES)?;

    prepare_credential_for_spending(
        &params,
        public_attributes,
        private_attributes,
        signature,
        verification_key,
    )
}
