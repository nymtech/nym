// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::default_database_filepath;
use crate::webhook::ZkNymWebHookConfig;
use clap::builder::ArgPredicate;
use clap::Parser;
use nym_bin_common::bin_info;
use std::fs::create_dir_all;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::info;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

// if needed this could be split into subcommands
#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
pub struct Cli {
    #[clap(flatten)]
    pub(crate) webhook: ZkNymWebHookConfig,

    /// Path pointing to an env file that configures the binary.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<PathBuf>,

    /// Specifies the custom port value used for the api server.
    /// default: `8080`
    #[clap(
        long,
        env = "NYM_CREDENTIAL_PROXY_PORT",
        default_value = "8080",
        default_value_if("bind_address", ArgPredicate::IsPresent, None)
    )]
    pub port: Option<u16>,

    /// Specifies the custom bind address value used for the api server.
    /// default: `0.0.0.0:8080`
    #[clap(long, env = "NYM_CREDENTIAL_PROXY_BIND_ADDRESS")]
    pub bind_address: Option<SocketAddr>,

    /// Specifies the mnemonic authorised for making deposits for "free pass" ticketbooks
    #[clap(long, env = "NYM_CREDENTIAL_PROXY_MNEMONIC")]
    pub mnemonic: bip39::Mnemonic,

    /// Bearer token for accessing the http endpoints.
    #[clap(long, env = "NYM_CREDENTIAL_PROXY_AUTH_TOKEN", alias = "http-bearer-token")]
    pub(crate) http_auth_token: String,

    #[clap(long, env = "NYM_CREDENTIAL_PROXY_PERSISTENT_STORAGE_STORAGE")]
    pub(crate) persistent_storage_path: Option<PathBuf>,
}

impl Cli {
    pub fn bind_address(&self) -> SocketAddr {
        // SAFETY:
        // if `bind_address` hasn't been specified, `port` will default to "8080",
        // so some value will always be available to use
        #[allow(clippy::unwrap_used)]
        self.bind_address.unwrap_or_else(|| {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.port.unwrap())
        })
    }

    pub fn persistent_storage_path(&self) -> PathBuf {
        self.persistent_storage_path.clone().unwrap_or_else(|| {
            // if this blows up, then we shouldn't continue
            #[allow(clippy::expect_used)]
            let default_path = default_database_filepath();
            if let Some(parent) = default_path.parent() {
                // make sure it exists
                #[allow(clippy::unwrap_used)]
                create_dir_all(parent).unwrap();
            }

            info!(
                "setting the storage path path to {}",
                default_path.display()
            );

            default_path
        })
    }
}
