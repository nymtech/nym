// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[derive(thiserror::Error, Debug)]
pub enum RegistrationClientError {
    #[error("failed to build mixnet client")]
    BuildMixnetClient(#[source] Box<nym_sdk::Error>),

    #[error("failed to connect to mixnet")]
    ConnectToMixnet(#[source] Box<nym_sdk::Error>),

    #[error("failed to connect to ip packet router")]
    ConnectToIpPacketRouter(#[source] nym_ip_packet_client::Error),

    #[error("the selected node does not have an IP packet router : {node_id}")]
    NoIpPacketRouterAddress { node_id: String },

    #[error(
        "wireguard authentication is not possible due to one of the gateways not running the authenticator process: {node_id} "
    )]
    AuthenticationNotPossible { node_id: String },

    #[error("Failed to create nyxd client config")]
    FailedToCreateNyxdClientConfig(nym_validator_client::nyxd::error::NyxdError),

    #[error("failed to parse nyxd_url")]
    InvalidNyxdUrl,

    #[error("Failed to connect using nyxd client")]
    FailedToConnectUsingNyxdClient(nym_validator_client::nyxd::error::NyxdError),

    #[error("connection cancelled")]
    Cancelled,

    #[error("timeout connecting the mixnet client")]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error(
        "failed to register wireguard with the gateway for {gateway_id}, no credential was sent"
    )]
    WireguardEntryRegistration {
        gateway_id: String,
        authenticator_address: Box<nym_sdk::mixnet::Recipient>,
        #[source]
        source: Box<nym_authenticator_client::AuthenticationClientError>,
    },

    #[error(
        "failed to register wireguard with the gateway for {gateway_id}, no credential was sent"
    )]
    WireguardExitRegistration {
        gateway_id: String,
        authenticator_address: Box<nym_sdk::mixnet::Recipient>,
        #[source]
        source: Box<nym_authenticator_client::AuthenticationClientError>,
    },

    #[error(
        "failed to register wireguard with the gateway for {gateway_id}, a credential was sent"
    )]
    WireguardEntryRegistrationCredentialSent {
        gateway_id: String,
        authenticator_address: Box<nym_sdk::mixnet::Recipient>,
        #[source]
        source: Box<nym_authenticator_client::AuthenticationClientError>,
    },

    #[error(
        "failed to register wireguard with the gateway for {gateway_id}, a credential was sent"
    )]
    WireguardExitRegistrationCredentialSent {
        gateway_id: String,
        authenticator_address: Box<nym_sdk::mixnet::Recipient>,
        #[source]
        source: Box<nym_authenticator_client::AuthenticationClientError>,
    },
}

impl RegistrationClientError {
    pub fn from_authenticator_error(
        error: nym_authenticator_client::RegistrationError,
        gateway_id: String,
        authenticator_address: nym_sdk::mixnet::Recipient,
        entry: bool,
    ) -> Self {
        match error {
            nym_authenticator_client::RegistrationError::NoCredentialSent(source) => {
                if entry {
                    Self::WireguardEntryRegistration {
                        gateway_id,
                        authenticator_address: Box::new(authenticator_address),
                        source: Box::new(source),
                    }
                } else {
                    Self::WireguardExitRegistration {
                        gateway_id,
                        authenticator_address: Box::new(authenticator_address),
                        source: Box::new(source),
                    }
                }
            }
            nym_authenticator_client::RegistrationError::CredentialSent { source } => {
                if entry {
                    Self::WireguardEntryRegistrationCredentialSent {
                        gateway_id,
                        authenticator_address: Box::new(authenticator_address),
                        source: Box::new(source),
                    }
                } else {
                    Self::WireguardExitRegistrationCredentialSent {
                        gateway_id,
                        authenticator_address: Box::new(authenticator_address),
                        source: Box::new(source),
                    }
                }
            }
        }
    }
}
