// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_bin_common::bin_info;
use nym_http_api_client::UserAgent;
use nym_validator_client::ecash::models::EcashSignerStatusResponseBody;
use nym_validator_client::nym_api::NymApiClientExt;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum::{Display, EnumProperty};
use time::{Duration, OffsetDateTime};
use tracing::error;

#[derive(Serialize, Deserialize)]
pub(crate) struct SignerStatus {
    api_endpoint: String,
    api_version: ApiVersion,
    rpc_status: RpcStatus,
    used_rpc_endpoint: RpcEndpoint,
    abci_version: AbciVersion,

    signer_status: Option<EcashSignerStatusResponseBody>,
}

impl Display for SignerStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        writeln!(f, "api_endpoint: {}", self.api_endpoint)?;
        writeln!(f, "api_version: {}", self.api_version)?;
        writeln!(f, "rpc_status: {}", self.rpc_status)?;
        writeln!(f, "used_rpc_endpoint: {}", self.used_rpc_endpoint)?;
        writeln!(f, "abci_version: {}", self.abci_version)?;
        writeln!(f, "can issue credentials: {}", self.can_sign())?;
        Ok(())
    }
}

impl SignerStatus {
    pub(crate) fn new(api_endpoint: String) -> Self {
        SignerStatus {
            api_endpoint,
            api_version: Default::default(),
            rpc_status: Default::default(),
            used_rpc_endpoint: Default::default(),
            abci_version: Default::default(),
            signer_status: None,
        }
    }

    pub(crate) fn can_sign(&self) -> bool {
        let Some(status) = &self.signer_status else {
            return false;
        };

        status.has_signing_keys && !status.signer_disabled && status.is_ecash_signer
    }

    pub(crate) fn api_up(&self) -> bool {
        matches!(self.api_version, ApiVersion::Available { .. })
    }

    pub(crate) fn rpc_up(&self) -> bool {
        matches!(self.rpc_status, RpcStatus::Up)
    }

    fn build_api_client(&self) -> Option<nym_http_api_client::Client> {
        let api_endpoint: nym_http_api_client::Url = match self.api_endpoint.as_str().parse() {
            Ok(endpoint) => endpoint,
            Err(err) => {
                error!("{} is not a valid api endpoint: {err}", self.api_endpoint);
                return None;
            }
        };

        nym_http_api_client::Client::builder(api_endpoint)
            .ok()?
            .no_hickory_dns()
            .with_timeout(std::time::Duration::from_secs(1))
            .with_user_agent(UserAgent::from(bin_info!()))
            .build()
            .ok()
    }

    pub(crate) async fn try_update_api_version(&mut self) {
        let Some(client) = self.build_api_client() else {
            return;
        };
        match client.build_information().await {
            Ok(build_info) => {
                self.api_version = ApiVersion::Available {
                    version: build_info.build_version,
                };
            }
            Err(err) => {
                error!(
                    "failed to retrieve build information of {}: {err}",
                    self.api_endpoint
                )
            }
        }
    }

    pub(crate) async fn try_update_signer_information(&mut self) {
        let Some(client) = self.build_api_client() else {
            return;
        };
        match client.get_signer_status().await {
            Ok(signer_status) => {
                self.signer_status = Some(signer_status.body);
            }
            Err(err) => {
                error!(
                    "failed to retrieve signer status of {}: {err}",
                    self.api_endpoint
                )
            }
        }
    }

    pub(crate) async fn try_update_rpc_status(&mut self) {
        let Some(client) = self.build_api_client() else {
            return;
        };

        match client.get_chain_status().await {
            Ok(chain_status) => {
                self.used_rpc_endpoint = RpcEndpoint(chain_status.connected_nyxd);
                let last_block =
                    OffsetDateTime::from(chain_status.status.latest_block.block.header.time);
                let now = OffsetDateTime::now_utc();
                let diff = now - last_block;
                if diff < Duration::minutes(2) {
                    self.rpc_status = RpcStatus::Up
                } else {
                    self.rpc_status = RpcStatus::Down
                }
                self.abci_version = AbciVersion::Available {
                    version: chain_status.status.abci.version,
                }
            }
            Err(err) => {
                error!(
                    "failed to retrieve chain status from {}: {err}",
                    self.api_endpoint
                );
            }
        }
    }

    pub(crate) fn to_table_row(&self) -> Vec<String> {
        vec![
            self.api_endpoint.to_string(),
            self.api_version.as_cell(),
            self.rpc_status.as_cell(),
            self.used_rpc_endpoint.as_cell(),
            self.abci_version.as_cell(),
            self.can_sign().to_string(),
        ]
    }
}

#[derive(Serialize, Deserialize, Default)]
struct RpcEndpoint(String);

impl Display for RpcEndpoint {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}

impl RpcEndpoint {
    fn as_cell(&self) -> String {
        if self.0.contains("localhost") || self.0.contains("127.0.0.1") {
            format!("✅ {}", self.0)
        } else if self.0.contains("nymtech") {
            format!("❗ {}", self.0)
        } else if self.0.is_empty() {
            "⚠️ unknown".to_string()
        } else {
            format!("⚠️  {}", self.0)
        }
    }
}

#[derive(
    Clone, Default, PartialOrd, PartialEq, Ord, Eq, Display, EnumProperty, Serialize, Deserialize,
)]
#[strum(serialize_all = "snake_case")]
enum AbciVersion {
    #[strum(props(emoji = "✅"))]
    #[strum(to_string = "{version}")]
    Available { version: String },

    #[strum(props(emoji = "❗"))]
    #[default]
    Unavailable,
}

impl AbciVersion {
    // SAFETY: every variant has a `emoji` prop defined
    #[allow(clippy::unwrap_used)]
    fn as_cell(&self) -> String {
        format!("{} {}", self.get_str("emoji").unwrap(), self)
    }
}

#[derive(
    Clone, Default, PartialOrd, PartialEq, Ord, Eq, Display, EnumProperty, Serialize, Deserialize,
)]
#[strum(serialize_all = "snake_case")]
enum ApiVersion {
    #[strum(props(emoji = "✅"))]
    #[strum(to_string = "{version}")]
    Available { version: String },

    #[strum(props(emoji = "❗"))]
    #[default]
    Unavailable,
}

impl ApiVersion {
    // SAFETY: every variant has a `emoji` prop defined
    #[allow(clippy::unwrap_used)]
    fn as_cell(&self) -> String {
        format!("{} {}", self.get_str("emoji").unwrap(), self)
    }
}

#[derive(
    Copy,
    Clone,
    Default,
    PartialOrd,
    PartialEq,
    Ord,
    Eq,
    Display,
    EnumProperty,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "snake_case")]
enum RpcStatus {
    #[strum(props(emoji = "❗"))]
    Down,

    #[strum(props(emoji = "✅"))]
    Up,

    #[strum(props(emoji = "⚠️"))]
    #[default]
    Unknown,
}

impl RpcStatus {
    // SAFETY: every variant has a `emoji` prop defined
    #[allow(clippy::unwrap_used)]
    fn as_cell(&self) -> String {
        format!("{} {}", self.get_str("emoji").unwrap(), self)
    }
}
