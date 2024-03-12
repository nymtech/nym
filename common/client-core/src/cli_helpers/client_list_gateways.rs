// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::{CliClient, CliClientConfig};
use crate::client::base_client::non_wasm_helpers::setup_fs_gateways_storage;
use crate::client::base_client::storage::helpers::get_gateway_registrations;
use nym_client_core_gateways_storage::{GatewayDetails, GatewayRegistration, GatewayType};
use nym_crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;
use url::Url;

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone)]
pub struct CommonClientListGatewaysArgs {
    /// Id of client we want to list gateways for.
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegisteredGateways(Vec<GatewayInfo>);

impl From<Vec<GatewayRegistration>> for RegisteredGateways {
    fn from(value: Vec<GatewayRegistration>) -> Self {
        RegisteredGateways(value.into_iter().map(Into::into).collect())
    }
}

impl Display for RegisteredGateways {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, gateway) in self.0.iter().enumerate() {
            writeln!(f, "[{i}]: {gateway}")?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct GatewayInfo {
    pub registration: OffsetDateTime,
    pub identity: identity::PublicKey,

    pub typ: String,
    pub endpoint: Option<Url>,
    pub wg_tun_address: Option<Url>,
}

impl From<GatewayRegistration> for GatewayInfo {
    fn from(value: GatewayRegistration) -> Self {
        match value.details {
            GatewayDetails::Remote(remote_details) => GatewayInfo {
                registration: value.registration_timestamp,
                identity: remote_details.gateway_id,
                typ: GatewayType::Remote.to_string(),
                endpoint: Some(remote_details.gateway_listener),
                wg_tun_address: remote_details.wg_tun_address,
            },
            GatewayDetails::Custom(_) => GatewayInfo {
                registration: value.registration_timestamp,
                identity: value.details.gateway_id(),
                typ: value.details.typ().to_string(),
                endpoint: None,
                wg_tun_address: None,
            },
        }
    }
}

impl Display for GatewayInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} gateway '{}' registered at: {}",
            self.typ, self.identity, self.registration
        )?;
        if let Some(endpoint) = &self.endpoint {
            write!(f, " endpoint: {endpoint}")?;
        }

        if let Some(wg_tun_address) = &self.wg_tun_address {
            write!(f, " wg tun address: {wg_tun_address}")?;
        }
        Ok(())
    }
}

pub async fn list_gateways<C, A>(args: A) -> Result<RegisteredGateways, C::Error>
where
    A: AsRef<CommonClientListGatewaysArgs>,
    C: CliClient,
{
    let common_args = args.as_ref();
    let id = &common_args.id;

    let config = C::try_load_current_config(id).await?;
    let paths = config.common_paths();

    let details_store = setup_fs_gateways_storage(&paths.gateway_registrations).await?;

    let gateways = get_gateway_registrations(&details_store).await?;

    Ok(gateways.into())
}
