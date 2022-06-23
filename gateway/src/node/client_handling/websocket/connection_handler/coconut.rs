// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_interface::VerificationKey;
use validator_client::{
    nymd::{NymdClient, SigningNymdClient},
    ApiClient,
};

pub struct CoconutVerifier {
    api_clients: Vec<ApiClient>,
    _nymd_client: NymdClient<SigningNymdClient>,
    aggregated_verification_key: VerificationKey,
}

impl CoconutVerifier {
    pub fn new(
        api_clients: Vec<ApiClient>,
        _nymd_client: NymdClient<SigningNymdClient>,
        aggregated_verification_key: VerificationKey,
    ) -> Self {
        CoconutVerifier {
            api_clients,
            _nymd_client,
            aggregated_verification_key,
        }
    }

    pub fn api_clients(&self) -> &Vec<ApiClient> {
        &self.api_clients
    }

    pub fn _nymd_client(&self) -> &NymdClient<SigningNymdClient> {
        &self._nymd_client
    }

    pub fn aggregated_verification_key(&self) -> &VerificationKey {
        &self.aggregated_verification_key
    }
}
