use std::time::Duration;

use nym_authenticator_client::{
    AuthenticatorClient, AuthenticatorResponse, AuthenticatorVersion, ClientMessage,
    QueryMessageImpl,
};
use nym_authenticator_requests::{v3, v4, v5};
use nym_credentials_interface::CredentialSpendingData;
use nym_crypto::asymmetric::encryption;
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::PeerPublicKey;
use nym_sdk::mixnet::Recipient;

use crate::error::{Error, Result};

const RETRY_PERIOD: Duration = Duration::from_secs(30);

impl crate::WgGatewayClient {
    pub fn light_client(&self) -> WgGatewayLightClient {
        WgGatewayLightClient {
            public_key: *self.keypair.public_key(),
            auth_client: self.auth_client.clone(),
        }
    }
}

#[derive(Clone)]
pub struct WgGatewayLightClient {
    public_key: encryption::PublicKey,
    auth_client: AuthenticatorClient,
}
impl WgGatewayLightClient {
    pub fn auth_recipient(&self) -> Recipient {
        self.auth_client.auth_recipient()
    }

    pub fn auth_client(&self) -> &AuthenticatorClient {
        &self.auth_client
    }

    pub fn set_auth_client(&mut self, auth_client: AuthenticatorClient) {
        self.auth_client = auth_client;
    }
    pub async fn query_bandwidth(&mut self) -> Result<Option<i64>> {
        let query_message = match self.auth_client.auth_version() {
            AuthenticatorVersion::V2 => ClientMessage::Query(Box::new(QueryMessageImpl {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                version: AuthenticatorVersion::V2,
            })),
            AuthenticatorVersion::V3 => ClientMessage::Query(Box::new(QueryMessageImpl {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                version: AuthenticatorVersion::V3,
            })),
            AuthenticatorVersion::V4 => ClientMessage::Query(Box::new(QueryMessageImpl {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                version: AuthenticatorVersion::V4,
            })),
            AuthenticatorVersion::V5 => ClientMessage::Query(Box::new(QueryMessageImpl {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                version: AuthenticatorVersion::V5,
            })),
            AuthenticatorVersion::UNKNOWN => return Err(Error::UnsupportedAuthenticatorVersion),
        };
        let response = self.auth_client.send(&query_message).await?;

        let available_bandwidth = match response {
            nym_authenticator_client::AuthenticatorResponse::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => {
                if let Some(available_bandwidth) =
                    remaining_bandwidth_response.available_bandwidth()
                {
                    available_bandwidth
                } else {
                    return Ok(None);
                }
            }
            _ => return Err(Error::InvalidGatewayAuthResponse),
        };

        let remaining_pretty = if available_bandwidth > 1024 * 1024 {
            format!("{:.2} MB", available_bandwidth as f64 / 1024.0 / 1024.0)
        } else {
            format!("{} KB", available_bandwidth / 1024)
        };
        tracing::debug!(
            "Remaining wireguard bandwidth with gateway {} for today: {}",
            self.auth_client.auth_recipient().gateway(),
            remaining_pretty
        );
        if available_bandwidth < 1024 * 1024 {
            tracing::warn!(
                "Remaining bandwidth is under 1 MB. The wireguard mode will get suspended after that until tomorrow, UTC time. The client might shutdown with timeout soon"
            );
        }
        Ok(Some(available_bandwidth))
    }
    async fn send(&mut self, msg: ClientMessage) -> Result<AuthenticatorResponse> {
        let now = std::time::Instant::now();
        while now.elapsed() < RETRY_PERIOD {
            match self.auth_client.send(&msg).await {
                Ok(response) => return Ok(response),
                Err(nym_authenticator_client::Error::TimeoutWaitingForConnectResponse) => continue,
                Err(source) => {
                    if msg.is_wasteful() {
                        return Err(Error::NoRetry { source });
                    } else {
                        return Err(Error::AuthenticatorClientError(source));
                    }
                }
            }
        }
        if msg.is_wasteful() {
            Err(Error::NoRetry {
                source: nym_authenticator_client::Error::TimeoutWaitingForConnectResponse,
            })
        } else {
            Err(Error::AuthenticatorClientError(
                nym_authenticator_client::Error::TimeoutWaitingForConnectResponse,
            ))
        }
    }

    pub async fn top_up(&mut self, credential: CredentialSpendingData) -> Result<i64> {
        let top_up_message = match self.auth_client.auth_version() {
            AuthenticatorVersion::V3 => ClientMessage::TopUp(Box::new(v3::topup::TopUpMessage {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                credential,
            })),
            // NOTE: looks like a bug here using v3. But we're leaving it as is since it's working
            // and V4 is deprecated in favour of V5
            AuthenticatorVersion::V4 => ClientMessage::TopUp(Box::new(v4::topup::TopUpMessage {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                credential,
            })),
            AuthenticatorVersion::V5 => ClientMessage::TopUp(Box::new(v5::topup::TopUpMessage {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                credential,
            })),
            AuthenticatorVersion::V2 | AuthenticatorVersion::UNKNOWN => {
                return Err(Error::UnsupportedAuthenticatorVersion);
            }
        };
        let response = self.send(top_up_message).await?;

        let remaining_bandwidth = match response {
            AuthenticatorResponse::TopUpBandwidth(top_up_bandwidth_response) => {
                top_up_bandwidth_response.available_bandwidth()
            }
            _ => return Err(Error::InvalidGatewayAuthResponse),
        };

        Ok(remaining_bandwidth)
    }
}
