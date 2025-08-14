// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::default_database_filepath;
use crate::error::VpnApiError;
use crate::webhook::ZkNymWebHookConfig;
use clap::builder::ArgPredicate;
use clap::{Args, Parser};
use nym_bin_common::bin_info;
use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::Ed25519RecoveryError;
use std::fs::create_dir_all;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::info;
use url::Url;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

// the reason for `Arc` is that `ArgMatches` impls `Clone`,
// so we also need to make the type clone-able
// https://github.com/clap-rs/clap/issues/4286#issuecomment-1262527218
#[derive(Debug, Clone)]
struct PrivateKeyCliWrapper(Arc<ed25519::PrivateKey>);

impl FromStr for PrivateKeyCliWrapper {
    type Err = Ed25519RecoveryError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PrivateKeyCliWrapper(Arc::new(s.parse()?)))
    }
}

#[derive(Debug, Args)]
#[clap(group = clap::ArgGroup::new("jwt-signing-keys").required(true).multiple(false))]
pub struct JwtSigningKeysArgs {
    /// Explicit base58-encoded ed25519 private key used for signing upgrade-mode jwt.
    #[clap(
        long,
        group = "jwt-signing-keys",
        env = "NYM_CREDENTIAL_PROXY_JWT_SIGNING_KEY"
    )]
    jwt_signing_key: Option<PrivateKeyCliWrapper>,

    /// Path to PEM file containing ed25519 private key used for signing upgrade-mode jwt.
    #[clap(
        long,
        group = "jwt-signing-keys",
        env = "NYM_CREDENTIAL_PROXY_JWT_SIGNING_KEY_PATH"
    )]
    jwt_signing_key_path: Option<PathBuf>,
}

impl JwtSigningKeysArgs {
    pub(crate) fn signing_keys(self) -> Result<ed25519::KeyPair, VpnApiError> {
        if let Some(key) = self.jwt_signing_key {
            // SAFETY: the arc has never been cloned
            #[allow(clippy::unwrap_used)]
            return Ok(Arc::into_inner(key.0).unwrap().into());
        }

        // SAFETY: due to clap group, clap ensures only one value here is set
        #[allow(clippy::unwrap_used)]
        let key_path = self.jwt_signing_key_path.unwrap();

        let key: ed25519::PrivateKey = nym_pemstore::load_key(&key_path).map_err(|err| {
            VpnApiError::JWTSigningKeyLoadFailure {
                path: key_path.to_str().map(|s| s.to_owned()).unwrap_or_default(),
                err,
            }
        })?;
        Ok(key.into())
    }
}

// if needed this could be split into subcommands
#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
pub struct Cli {
    #[clap(flatten)]
    pub(crate) webhook: ZkNymWebHookConfig,

    #[clap(flatten)]
    pub(crate) jwt_signing_keys: JwtSigningKeysArgs,

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
    #[clap(
        long,
        env = "NYM_CREDENTIAL_PROXY_AUTH_TOKEN",
        alias = "http-bearer-token"
    )]
    pub(crate) http_auth_token: String,

    /// Specify the maximum number of deposits the credential proxy can make in a single transaction
    /// (default: 32)
    #[clap(
        long,
        env = "NYM_CREDENTIAL_PROXY_MAX_CONCURRENT_DEPOSITS",
        default_value_t = 32
    )]
    pub(crate) max_concurrent_deposits: usize,

    #[clap(
        long,
        value_parser = humantime::parse_duration,
        env = "NYM_CREDENTIAL_PROXY_ATTESTATION_CHECK_REGULAR_POLLING_INTERVAL",
        default_value = "5m",
    )]
    pub(crate) attestation_check_url: Url,

    #[clap(
        long,
        value_parser = humantime::parse_duration,
        env = "NYM_CREDENTIAL_PROXY_ATTESTATION_CHECK_REGULAR_POLLING_INTERVAL",
        default_value = "5m",
    )]
    pub(crate) attestation_check_regular_polling_interval: Duration,

    #[clap(
        long,
        value_parser = humantime::parse_duration,
        env = "NYM_CREDENTIAL_PROXY_ATTESTATION_CHECK_EXPEDITED_POLLING_INTERVAL",
        default_value = "1m",
    )]
    pub(crate) attestation_check_expedited_polling_interval: Duration,

    #[clap(
        long,
        value_parser = humantime::parse_duration,
        env = "NYM_CREDENTIAL_PROXY_UPGRADE_MODE_JWT_VALIDITY",
        default_value = "1h",
    )]
    pub(crate) upgrade_mode_jwt_validity: Duration,

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

            info!("setting the storage path to {}", default_path.display());

            default_path
        })
    }
}
