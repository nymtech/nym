// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::AuthenticatorError;
use crate::cli::{override_config, OverrideConfig};
use crate::cli::{try_load_current_config, version_check};
use clap::{Args, Subcommand};
use nym_authenticator_requests::latest::{
    registration::{ClientMac, FinalMessage, GatewayClient, InitMessage},
    request::{AuthenticatorRequest, AuthenticatorRequestData},
};
use nym_client_core::cli_helpers::client_run::CommonClientRunArgs;
use nym_sdk::mixnet::{MixnetMessageSender, Recipient, TransmissionLane};
use nym_task::TaskHandle;
use nym_wireguard_types::PeerPublicKey;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Clone)]
pub(crate) struct Request {
    #[command(flatten)]
    common_args: CommonClientRunArgs,

    #[command(subcommand)]
    request: RequestType,

    authenticator_recipient: String,
}

impl From<Request> for OverrideConfig {
    fn from(request_config: Request) -> Self {
        OverrideConfig {
            nym_apis: None,
            nyxd_urls: request_config.common_args.nyxd_urls,
            enabled_credentials_mode: request_config.common_args.enabled_credentials_mode,
        }
    }
}

#[derive(Clone, Subcommand)]
pub(crate) enum RequestType {
    Initial(Initial),
    Final(Final),
    QueryBandwidth(QueryBandwidth),
}

#[derive(Args, Clone, Debug)]
pub(crate) struct Initial {
    pub_key: String,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct Final {
    pub_key: String,
    private_ip: String,
    mac: String,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct QueryBandwidth {
    pub_key: String,
}

impl TryFrom<RequestType> for AuthenticatorRequestData {
    type Error = AuthenticatorError;

    fn try_from(value: RequestType) -> Result<Self, Self::Error> {
        let ret = match value {
            RequestType::Initial(req) => AuthenticatorRequestData::Initial(InitMessage::new(
                PeerPublicKey::from_str(&req.pub_key)?,
            )),
            RequestType::Final(req) => AuthenticatorRequestData::Final(Box::new(FinalMessage {
                gateway_client: GatewayClient {
                    pub_key: PeerPublicKey::from_str(&req.pub_key)?,
                    private_ip: IpAddr::from_str(&req.private_ip)?,
                    mac: ClientMac::from_str(&req.mac)?,
                },
                credential: None,
            })),
            RequestType::QueryBandwidth(req) => {
                AuthenticatorRequestData::QueryBandwidth(PeerPublicKey::from_str(&req.pub_key)?)
            }
        };
        Ok(ret)
    }
}

pub(crate) async fn execute(args: &Request) -> Result<(), AuthenticatorError> {
    let mut config = try_load_current_config(&args.common_args.id).await?;
    config = override_config(config, OverrideConfig::from(args.clone()));

    if !version_check(&config) {
        log::error!("failed the local version check");
        return Err(AuthenticatorError::FailedLocalVersionCheck);
    }

    let shutdown = TaskHandle::default();
    let mixnet_client = nym_authenticator::mixnet_client::create_mixnet_client(
        &config.base,
        shutdown.get_handle().named("nym_sdk::MixnetClient"),
        None,
        None,
        false,
        &config.storage_paths.common_paths,
    )
    .await?;

    let request_data = AuthenticatorRequestData::try_from(args.request.clone())?;
    let authenticator_recipient = Recipient::from_str(&args.authenticator_recipient)?;
    let (request, _) = match request_data {
        AuthenticatorRequestData::Initial(init_message) => {
            AuthenticatorRequest::new_initial_request(init_message, *mixnet_client.nym_address())
        }
        AuthenticatorRequestData::Final(final_message) => {
            AuthenticatorRequest::new_final_request(*final_message, *mixnet_client.nym_address())
        }
        AuthenticatorRequestData::QueryBandwidth(query_message) => {
            AuthenticatorRequest::new_query_request(query_message, *mixnet_client.nym_address())
        }
        AuthenticatorRequestData::TopUpBandwidth(top_up_message) => {
            AuthenticatorRequest::new_topup_request(*top_up_message, *mixnet_client.nym_address())
        }
    };
    mixnet_client
        .split_sender()
        .send(nym_sdk::mixnet::InputMessage::new_regular(
            authenticator_recipient,
            request.to_bytes().unwrap(),
            TransmissionLane::General,
            None,
        ))
        .await
        .map_err(|source| AuthenticatorError::FailedToSendPacketToMixnet { source })?;

    log::info!("Sent request, sleeping 60 seconds or until killed");
    sleep(Duration::from_secs(60)).await;

    Ok(())
}
