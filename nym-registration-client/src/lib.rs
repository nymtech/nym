// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use builder::RegistrationClientBuilder;
pub use builder::config::{
    BuilderConfig as RegistrationClientBuilderConfig, MixnetClientConfig,
    NymNodeWithKeys as RegistrationNymNode,
};
pub use config::RegistrationMode;
pub use error::RegistrationClientError;
pub use lp_client::{
    LpRegistrationClient, LpRegistrationConfig, NestedLpSession, error::LpClientError,
};
pub use types::{
    LpRegistrationResult, MixnetRegistrationResult, RegistrationResult, WireguardRegistrationResult,
};

mod builder;
mod clients;
mod config;
mod error;
mod lp_client;
mod types;

pub enum RegistrationClient {
    Mixnet(Box<clients::MixnetBasedRegistrationClient>),
    Lp(Box<clients::LpBasedRegistrationClient>),
}

impl RegistrationClient {
    pub async fn register(self) -> Result<RegistrationResult, RegistrationClientError> {
        match self {
            RegistrationClient::Mixnet(client) => client.register().await,
            RegistrationClient::Lp(client) => client.register().await,
        }
    }
}
