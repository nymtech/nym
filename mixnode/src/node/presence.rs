// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use validator_client::models::mixnode::MixRegistrationInfo;
use validator_client::ValidatorClientError;

// there's no point in keeping the validator client persistently as it might be literally hours or days
// before it's used again
pub(crate) async fn register_with_validator(
    validator_endpoint: String,
    mix_host: String,
    identity_key: String,
    sphinx_key: String,
    version: String,
    location: String,
    layer: u64,
    incentives_address: Option<String>,
) -> Result<(), ValidatorClientError> {
    let config = validator_client::Config::new(validator_endpoint);
    let validator_client = validator_client::Client::new(config);

    let registration_info = MixRegistrationInfo::new(
        mix_host,
        identity_key,
        sphinx_key,
        version,
        location,
        layer,
        incentives_address,
    );

    validator_client.register_mix(registration_info).await
}

pub(crate) async fn unregister_with_validator(
    validator_endpoint: String,
    identity_key: String,
) -> Result<(), ValidatorClientError> {
    let config = validator_client::Config::new(validator_endpoint);
    let validator_client = validator_client::Client::new(config);

    validator_client.unregister_node(&identity_key).await
}
