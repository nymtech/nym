// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::{
    ConfigArgs, EntryGatewayArgs, ExitGatewayArgs, HostArgs, HttpArgs, MetricsArgs, MixnetArgs,
    VerlocArgs, WireguardArgs,
};
use crate::config::persistence::NymNodePaths;
use crate::config::{Config, ConfigBuilder, NodeMode, NodeModes};
use crate::env::vars::*;
use crate::error::NymNodeError;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use zeroize::Zeroizing;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    pub(crate) config: ConfigArgs,

    /// Explicitly specify whether you agree with the terms and conditions of a nym node operator
    /// as defined at <https://nymtech.net/terms-and-conditions/operators/v1.0.0>
    #[clap(
        long,
        env = NYMNODE_ACCEPT_OPERATOR_TERMS,
        alias = "accept-t&c",
        alias = "accept-operator-terms",
        alias = "accept-operator-t&c",
    )]
    pub(crate) accept_operator_terms_and_conditions: bool,

    /// Forbid a new node from being initialised if configuration file for the provided specification doesn't already exist
    #[clap(
        long,
        default_value_t = false,
        env = NYMNODE_DENY_INIT_ARG,
        conflicts_with = "init_only"
    )]
    pub(crate) deny_init: bool,

    /// If this is a brand new nym-node, specify whether it should only be initialised without actually running the subprocesses.
    #[clap(
        long,
        default_value_t = false,
        env = NYMNODE_INIT_ONLY_ARG,
        conflicts_with = "deny_init"
    )]
    pub(crate) init_only: bool,

    /// Flag specifying this node will be running in a local setting.
    #[clap(
        long,
        default_value_t = false,
        env = NYMNODE_LOCAL_ARG
    )]
    pub(crate) local: bool,

    /// Specifies the current mode(s) of this nym-node.
    #[clap(
        long,
        value_enum,
        env = NYMNODE_MODE_ARG,
        num_args(0..=3),
        group = "node_mode"
    )]
    pub(crate) mode: Option<Vec<NodeMode>>,

    /// Specifies the current mode(s) of this nym-node as a single flag.
    #[clap(
        long,
        value_enum,
        env = NYMNODE_MODES_ARG,
        value_delimiter = ',',
        group = "node_mode"
    )]
    pub(crate) modes: Vec<NodeMode>,

    /// If this node has been initialised before, specify whether to write any new changes to the config file.
    #[clap(
        short,
        long,
        default_value_t = false,
        env = NYMMONDE_WRITE_CONFIG_CHANGES_ARG,
    )]
    pub(crate) write_changes: bool,

    /// Specify output file for bonding information of this nym-node, i.e. its encoded keys.
    /// NOTE: the required bonding information is still a subject to change and this argument should be treated
    /// only as a preview of future features.
    #[clap(
        long,
        env = NYMNODE_BONDING_INFORMATION_OUTPUT_ARG
    )]
    pub(crate) bonding_information_output: Option<PathBuf>,

    /// Specify the output format of the bonding information (`text` or `json`)
    #[clap(
        short,
        long,
        default_value_t = OutputFormat::default(),
        env = NYMNODE_OUTPUT_ARG
    )]
    pub(crate) output: OutputFormat,

    #[clap(flatten)]
    host: HostArgs,

    #[clap(flatten)]
    http: HttpArgs,

    #[clap(flatten)]
    mixnet: MixnetArgs,

    #[clap(flatten)]
    metrics: MetricsArgs,

    #[clap(flatten)]
    wireguard: WireguardArgs,

    #[clap(flatten)]
    verloc: VerlocArgs,

    #[clap(flatten)]
    entry_gateway: EntryGatewayArgs,

    #[clap(flatten)]
    exit_gateway: ExitGatewayArgs,
}

impl Args {
    pub(super) fn take_mnemonic(&mut self) -> Option<Zeroizing<bip39::Mnemonic>> {
        self.entry_gateway.mnemonic.take().map(Zeroizing::new)
    }
}

impl Args {
    pub(crate) fn custom_modes(&self) -> Option<NodeModes> {
        if let Some(explicit_modes) = &self.mode {
            return Some(explicit_modes.as_slice().into());
        }

        if !self.modes.is_empty() {
            return Some(self.modes.as_slice().into());
        }

        None
    }

    pub(crate) fn build_config(self) -> Result<Config, NymNodeError> {
        let config_path = self.config.config_path();
        let data_dir = Config::default_data_directory(&config_path)?;

        let id = self
            .config
            .id()
            .clone()
            .ok_or(NymNodeError::MissingInitArg {
                section: "global".to_string(),
                name: "id".to_string(),
            })?;

        let config = ConfigBuilder::new(id, config_path.clone(), data_dir.clone())
            // the old default behaviour of running in mixnode mode if nothing is explicitly set
            .with_modes(
                self.custom_modes()
                    .unwrap_or(*NodeModes::default().with_mixnode()),
            )
            .with_host(self.host.build_config_section())
            .with_http(self.http.build_config_section())
            .with_mixnet(self.mixnet.build_config_section(&data_dir))
            .with_wireguard(self.wireguard.build_config_section(&data_dir))
            .with_storage_paths(NymNodePaths::new(&data_dir))
            .with_verloc(self.verloc.build_config_section())
            .with_metrics(self.metrics.build_config_section())
            .with_gateway_tasks(self.entry_gateway.build_config_section(&data_dir))
            .with_service_providers(self.exit_gateway.build_config_section(&data_dir))
            .build();

        Ok(config)
    }

    pub(crate) fn override_config(self, mut config: Config) -> Config {
        if let Some(modes) = self.custom_modes() {
            config.modes = modes;
        }

        config.host = self.host.override_config_section(config.host);
        config.http = self.http.override_config_section(config.http);
        config.mixnet = self.mixnet.override_config_section(config.mixnet);
        config.wireguard = self.wireguard.override_config_section(config.wireguard);
        config.metrics = self.metrics.override_config_section(config.metrics);
        config.gateway_tasks = self
            .entry_gateway
            .override_config_section(config.gateway_tasks);
        config.service_providers = self
            .exit_gateway
            .override_config_section(config.service_providers);
        config
    }
}
