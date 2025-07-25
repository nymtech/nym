// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::env::vars::*;
use crate::monitor::SignersMonitor;
use anyhow::{bail, Context};
use clap::ArgGroup;
use nym_network_defaults::{setup_env, NymNetworkDetails};
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::time::Duration;
use url::Url;

#[derive(Debug, clap::Args)]
pub(crate) struct NyxdConnectionArgs {
    // for well-known networks, such mainnet, we can use hardcoded values
    /// Name of a well known network (such as 'mainnet') that has well-known
    /// pre-configured setup values
    #[clap(long, env = KNOWN_NETWORK_NAME_ARG)]
    pub(crate) known_network_name: Option<String>,

    /// Path pointing to an env file that configures the nyxd client.
    #[clap(
        short,
        long,
        env = NYXD_CLIENT_CONFIG_ENV_FILE_ARG
    )]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// For unknown networks (or if one wishes to override defaults),
    /// specify the RPC endpoint of a node from which signer information should be retrieved
    #[clap(long, env = NYXD_RPC_ENDPOINT_ARG)]
    pub(crate) nyxd_rpc_endpoint: Option<Url>,

    /// For unknown networks, specify address of the DKG contract to pull signer information from.
    #[clap(
        long,
        requires("nyxd_rpc_endpoint"),
        env = NYXD_DKG_CONTRACT_ADDRESS_ARG
    )]
    pub(crate) dkg_contract_address: Option<AccountId>,
    // if needed down the line (not sure why), we could define additional args
    // for specifying denoms, etc.
    // #[clap(long, requires("dkg_contract_address"))]
    // pub(crate) mix_denom: Option<String>,
}

impl NyxdConnectionArgs {
    fn get_minimal_nym_network_details(&self) -> anyhow::Result<NymNetworkDetails> {
        if let Some(known_network_name) = &self.known_network_name {
            match known_network_name.as_str() {
                "mainnet" => return Ok(NymNetworkDetails::new_mainnet()),
                other => bail!("{other} is not a known network name - please use another method of setting up chain connection"),
            }
        }

        if let Some(config_env_file) = &self.config_env_file {
            setup_env(Some(config_env_file));
            return Ok(NymNetworkDetails::new_from_env());
        }

        // SAFETY: clap ensures at least one of the fields is set
        #[allow(clippy::unwrap_used)]
        let dkg_contract = self.dkg_contract_address.as_ref().unwrap();

        // use mainnet's chain details (i.e. prefixes, denoms, etc)
        let mainnet_chain_details = NymNetworkDetails::new_mainnet().chain_details;
        Ok(NymNetworkDetails::new_empty()
            .with_chain_details(mainnet_chain_details)
            .with_coconut_dkg_contract(Some(dkg_contract.to_string())))
    }

    pub(crate) fn try_create_nyxd_client(&self) -> anyhow::Result<QueryHttpRpcNyxdClient> {
        let network_details = self.get_minimal_nym_network_details()?;

        let nyxd_endpoint = match &self.nyxd_rpc_endpoint {
            Some(nyxd_rpc_endpoint) => nyxd_rpc_endpoint.clone(),
            None => network_details
                .endpoints
                .first()
                .context("no nyxd endpoints provided")?
                .nyxd_url
                .parse()?,
        };

        Ok(QueryHttpRpcNyxdClient::connect_with_network_details(
            nyxd_endpoint.as_str(),
            network_details,
        )?)
    }
}

#[derive(clap::Args, Debug)]
#[command(group(
    ArgGroup::new("nyxd_connection")
        .multiple(true)
        .required(true)
        .args([
            "nyxd_connection.known_network_name", 
            "nyxd_connection.config_env_file",
            "nyxd_connection.nyxd_rpc_endpoint"
        ])
))]
pub(crate) struct Args {
    /// Specify email address for the bot responsible for sending notifications to the zulip server
    /// in case 'upgrade' mode is detected
    #[clap(
        long,
        env = ZULIP_BOT_EMAIL_ARG
    )]
    pub(crate) zulip_bot_email: String,

    /// Specify the API key for the bot responsible for sending notifications to the zulip server
    /// in case 'upgrade' mode is detected
    #[clap(
        long,
        env = ZULIP_BOT_API_KEY_ARG
    )]
    pub(crate) zulip_bot_api_key: String,

    /// Specify the sever endpoint for the bot responsible for sending notifications
    /// in case 'upgrade' mode is detected
    #[clap(
        long,
        env = ZULIP_SERVER_URL_ARG
    )]
    pub(crate) zulip_server_url: Url,

    /// Specify the channel id for where the notification is going to be sent
    #[clap(
        long,
        env = ZULIP_NOTIFICATION_CHANNEL_ID_ARG
    )]
    pub(crate) zulip_notification_channel_id: u32,

    /// Specify the delay between subsequent signers checks
    #[clap(
        long,
        env = SIGNERS_MONITOR_CHECK_INTERVAL_ARG,
        value_parser = humantime::parse_duration,
        default_value = "15m"
    )]
    pub(crate) signers_check_interval: Duration,

    #[clap(flatten)]
    pub(crate) nyxd_connection: NyxdConnectionArgs,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    SignersMonitor::new(args)?.run().await
}
