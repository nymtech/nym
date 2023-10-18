// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::default_config_filepath;
use crate::config::persistence::paths::GatewayPaths;
use crate::config::{Config, Debug, Gateway, NetworkRequester};
use nym_bin_common::logging::LoggingSettings;
use nym_config::read_config_from_toml_file;
use nym_network_defaults::DEFAULT_HTTP_API_LISTENING_PORT;
use serde::{Deserialize, Serialize};
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_30 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    pub gateway: GatewayV1_1_30,

    pub storage_paths: GatewayPaths,

    pub network_requester: NetworkRequester,

    #[serde(default)]
    pub logging: LoggingSettings,

    #[serde(default)]
    pub debug: Debug,
}

impl ConfigV1_1_30 {
    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        read_config_from_toml_file(default_config_filepath(id))
    }
}

impl From<ConfigV1_1_30> for Config {
    fn from(value: ConfigV1_1_30) -> Self {
        Config {
            save_path: value.save_path,
            gateway: Gateway {
                version: value.gateway.version,
                id: value.gateway.id,
                only_coconut_credentials: value.gateway.only_coconut_credentials,
                listening_address: value.gateway.listening_address,
                mix_port: value.gateway.mix_port,
                http_api_port: DEFAULT_HTTP_API_LISTENING_PORT,
                clients_port: value.gateway.clients_port,
                enabled_statistics: value.gateway.enabled_statistics,
                nym_api_urls: value.gateway.nym_api_urls,
                nyxd_urls: value.gateway.nyxd_urls,
                statistics_service_url: value.gateway.statistics_service_url,
                cosmos_mnemonic: value.gateway.cosmos_mnemonic,
            },
            storage_paths: value.storage_paths,
            network_requester: value.network_requester,
            logging: value.logging,
            debug: value.debug,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GatewayV1_1_30 {
    /// Version of the gateway for which this configuration was created.
    pub version: String,

    /// ID specifies the human readable ID of this particular gateway.
    pub id: String,

    /// Indicates whether this gateway is accepting only coconut credentials for accessing the
    /// the mixnet, or if it also accepts non-paying clients
    #[serde(default)]
    pub only_coconut_credentials: bool,

    /// Address to which this mixnode will bind to and will be listening for packets.
    pub listening_address: IpAddr,

    /// Port used for listening for all mixnet traffic.
    /// (default: 1789)
    pub mix_port: u16,

    /// Port used for listening for all client-related traffic.
    /// (default: 9000)
    pub clients_port: u16,

    /// Whether gateway collects and sends anonymized statistics
    pub enabled_statistics: bool,

    /// Domain address of the statistics service
    pub statistics_service_url: Url,

    /// Addresses to APIs from which the node gets the view of the network.
    #[serde(alias = "validator_api_urls")]
    pub nym_api_urls: Vec<Url>,

    /// Addresses to validators which the node uses to check for double spending of ERC20 tokens.
    #[serde(alias = "validator_nymd_urls")]
    pub nyxd_urls: Vec<Url>,

    /// Mnemonic of a cosmos wallet used in checking for double spending.
    // #[deprecated(note = "move to storage")]
    // TODO: I don't think this should be stored directly in the config...
    pub cosmos_mnemonic: bip39::Mnemonic,
}
