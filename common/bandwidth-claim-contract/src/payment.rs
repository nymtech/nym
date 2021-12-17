// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::keys::{PublicKey, Signature};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct Payment {
    verification_key: PublicKey,
    gateway_identity: PublicKey,
    bandwidth: u64,
}

impl Payment {
    pub fn new(verification_key: PublicKey, gateway_identity: PublicKey, bandwidth: u64) -> Self {
        Payment {
            verification_key,
            gateway_identity,
            bandwidth,
        }
    }

    pub fn verification_key(&self) -> PublicKey {
        self.verification_key
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LinkPaymentData {
    pub verification_key: PublicKey,
    pub gateway_identity: PublicKey,
    pub bandwidth: u64,
    pub signature: Signature,
}

impl LinkPaymentData {
    pub fn new(
        verification_key: [u8; 32],
        gateway_identity: [u8; 32],
        bandwidth: u64,
        signature: [u8; 64],
    ) -> Self {
        LinkPaymentData {
            verification_key: PublicKey::new(verification_key),
            gateway_identity: PublicKey::new(gateway_identity),
            bandwidth,
            signature: Signature::new(signature),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct PagedPaymentResponse {
    pub payments: Vec<Payment>,
    pub per_page: usize,
    pub start_next_after: Option<PublicKey>,
}

impl PagedPaymentResponse {
    pub fn new(
        payments: Vec<Payment>,
        per_page: usize,
        start_next_after: Option<PublicKey>,
    ) -> Self {
        PagedPaymentResponse {
            payments,
            per_page,
            start_next_after,
        }
    }
}
