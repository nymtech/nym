// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_interface::VerificationKey;
use validator_client::ApiClient;

pub struct CoconutVerifier {
    api_client: ApiClient,
    aggregated_verification_key: VerificationKey,
}

impl CoconutVerifier {
    pub fn new(api_client: ApiClient, aggregated_verification_key: VerificationKey) -> Self {
        CoconutVerifier {
            api_client,
            aggregated_verification_key,
        }
    }

    pub fn api_client(&self) -> &ApiClient {
        &self.api_client
    }

    pub fn aggregated_verification_key(&self) -> &VerificationKey {
        &self.aggregated_verification_key
    }
}
