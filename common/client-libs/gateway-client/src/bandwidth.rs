// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "coconut")]
use coconut_interface::Credential;
#[cfg(feature = "coconut")]
use credentials::{bandwidth::prepare_for_spending, obtain_aggregate_verification_key};
#[cfg(feature = "coconut")]
use crypto::asymmetric::identity::PublicKey;
#[cfg(feature = "coconut")]
use url::Url;

#[derive(Clone)]
pub struct BandwidthController {
    #[cfg(feature = "coconut")]
    validator_endpoints: Vec<Url>,
    #[cfg(feature = "coconut")]
    identity: PublicKey,
}

impl BandwidthController {
    #[cfg(feature = "coconut")]
    pub fn new(validator_endpoints: Vec<Url>, identity: PublicKey) -> Self {
        BandwidthController {
            validator_endpoints,
            identity,
        }
    }

    #[cfg(not(feature = "coconut"))]
    pub fn new() -> Self {
        BandwidthController {}
    }

    #[cfg(feature = "coconut")]
    pub async fn prepare_coconut_credential(&self) -> Credential {
        let verification_key = obtain_aggregate_verification_key(&self.validator_endpoints)
            .await
            .expect("could not obtain aggregate verification key of validators");

        let bandwidth_credential = credentials::bandwidth::obtain_signature(
            &self.identity.to_bytes(),
            &self.validator_endpoints,
        )
        .await
        .expect("could not obtain bandwidth credential");
        // the above would presumably be loaded from a file

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        prepare_for_spending(
            &self.identity.to_bytes(),
            &bandwidth_credential,
            &verification_key,
        )
        .expect("could not prepare out bandwidth credential for spending")
    }
}
