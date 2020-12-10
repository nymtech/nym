// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use validator_client::models::gateway::GatewayRegistrationInfo;
use validator_client::ValidatorClientError;

// there's no point in keeping the validator client persistently as it might be literally hours or days
// before it's used again
pub(crate) async fn register_with_validator(
    validator_endpoint: String,
    mix_host: String,
    clients_host: String,
    identity_key: String,
    sphinx_key: String,
    version: String,
    location: String,
    incentives_address: Option<String>,
) -> Result<(), ValidatorClientError> {
    let config = validator_client::Config::new(validator_endpoint);
    let validator_client = validator_client::Client::new(config);

    let registration_info = GatewayRegistrationInfo::new(
        mix_host,
        clients_host,
        identity_key,
        sphinx_key,
        version,
        location,
        incentives_address,
    );

    validator_client.register_gateway(registration_info).await
}

pub(crate) async fn unregister_with_validator(
    validator_endpoint: String,
    identity_key: String,
) -> Result<(), ValidatorClientError> {
    let config = validator_client::Config::new(validator_endpoint);
    let validator_client = validator_client::Client::new(config);

    validator_client.unregister_node(&identity_key).await
}
