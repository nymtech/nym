// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail};
use cosmrs::AccountId;
use cosmwasm_std::{Addr, Coin as CosmWasmCoin, Decimal};
use log::error;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_validator_client::nyxd::Coin;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

// TODO: perhaps it should be moved to some global common crate?
pub fn account_id_to_cw_addr(account_id: &AccountId) -> Addr {
    // the call to unchecked is fine here as we're converting directly from `AccountId`
    // which must have been a valid bech32 address
    Addr::unchecked(account_id.as_ref())
}

pub fn pretty_coin(coin: &Coin) -> String {
    let amount = Decimal::from_ratio(coin.amount, 1_000_000u128);
    let denom = if coin.denom.starts_with('u') {
        &coin.denom[1..]
    } else {
        &coin.denom
    };
    format!("{amount} {denom}")
}

pub fn pretty_cosmwasm_coin(coin: &CosmWasmCoin) -> String {
    let amount = Decimal::from_ratio(coin.amount, 1_000_000u128);
    let denom = if coin.denom.starts_with('u') {
        &coin.denom[1..]
    } else {
        &coin.denom
    };
    format!("{amount} {denom}")
}

pub fn pretty_decimal_with_denom(value: Decimal, denom: &str) -> String {
    // TODO: we might have to truncate the value here (that's why I moved it to separate function)
    format!("{value} {denom}")
}

pub fn show_error<E>(e: E)
where
    E: Display,
{
    error!("{}", e);
}

pub fn show_error_passthrough<E>(e: E) -> E
where
    E: Error + Display,
{
    error!("{}", e);
    e
}

#[derive(Serialize)]
pub(crate) struct DataWrapper<T> {
    data: T,
}

impl<T> Display for DataWrapper<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl<T> DataWrapper<T> {
    pub(crate) fn new(data: T) -> Self {
        DataWrapper { data }
    }
}

fn find_toml_value<'a>(root: &'a toml::Value, key: &str) -> Option<&'a toml::Value> {
    if let toml::Value::Table(table) = root {
        for (k, v) in table {
            if k == key {
                return Some(v);
            }
            if v.is_table() {
                if let Some(res) = find_toml_value(v, key) {
                    return Some(res);
                }
            }
        }
    }
    None
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum CommonConfigsWrapper {
    // native, socks5, NR, etc. clients
    NymClients(Box<ClientConfigCommonWrapper>),

    // nym-api
    NymApi(NymApiConfigLight),

    // anything else that might get get introduced
    Unknown(UnknownConfigWrapper),
}

impl CommonConfigsWrapper {
    pub(crate) fn try_load<P: AsRef<Path>>(path: P) -> anyhow::Result<CommonConfigsWrapper> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub(crate) fn try_get_id(&self) -> anyhow::Result<&str> {
        match self {
            CommonConfigsWrapper::NymClients(cfg) => cfg.try_get_id(),
            CommonConfigsWrapper::NymApi(cfg) => Ok(&cfg.base.id),
            CommonConfigsWrapper::Unknown(cfg) => cfg.try_get_id(),
        }
    }

    pub(crate) fn try_get_private_id_key(&self) -> anyhow::Result<PathBuf> {
        match self {
            CommonConfigsWrapper::NymClients(cfg) => Ok(cfg
                .storage_paths
                .inner
                .keys
                .private_identity_key_file
                .clone()),
            CommonConfigsWrapper::NymApi(_cfg) => {
                todo!() //SW this will depend on the new network monitor structure. Ping @Drazen
            }
            CommonConfigsWrapper::Unknown(cfg) => cfg.try_get_private_id_key(),
        }
    }

    pub(crate) fn try_get_credentials_store(&self) -> anyhow::Result<PathBuf> {
        match self {
            CommonConfigsWrapper::NymClients(cfg) => {
                Ok(cfg.storage_paths.inner.credentials_database.clone())
            }
            CommonConfigsWrapper::NymApi(cfg) => Ok(cfg
                .network_monitor
                .storage_paths
                .credentials_database_path
                .clone()),
            CommonConfigsWrapper::Unknown(cfg) => cfg.try_get_credentials_store(),
        }
    }
}

// ideally we would have just imported the full nym-api config structure, but that'd have been an overkill,
// because we'd have to import the whole crate
#[derive(Deserialize, Debug)]
pub(crate) struct NymApiConfigLight {
    base: NymApiConfigBaseLight,
    network_monitor: NymApiConfigNetworkMonitorLight,
}

#[derive(Deserialize, Debug)]
struct NymApiConfigBaseLight {
    id: String,
}

#[derive(Deserialize, Debug)]
struct NymApiConfigNetworkMonitorLight {
    storage_paths: NetworkMonitorPaths,
}

#[derive(Deserialize, Debug)]
struct NetworkMonitorPaths {
    credentials_database_path: PathBuf,
}

// a hacky way of reading common data from client configs (native, socks5, etc.)
// it works because all clients follow the same structure for storage paths
// (or so I thought)
#[derive(Deserialize, Debug)]
pub(crate) struct ClientConfigCommonWrapper {
    storage_paths: StoragePathsWrapper,

    // ... but they have different structure for `nym_client_core::config::Client`
    // native client has it on the top layer, whilsts socks5 has it under 'core' table
    #[serde(flatten)]
    other: toml::Value,
}

// wrapper to allow for any additional entries besides the common paths, like allow list for NR
#[derive(Deserialize, Debug)]
struct StoragePathsWrapper {
    #[serde(flatten)]
    inner: CommonClientPaths,
}

impl ClientConfigCommonWrapper {
    pub(crate) fn try_get_id(&self) -> anyhow::Result<&str> {
        let id_val = find_toml_value(&self.other, "id")
            .ok_or_else(|| anyhow!("no id field present in the config"))?;
        if let toml::Value::String(id) = id_val {
            Ok(id)
        } else {
            bail!("no id field present in the config")
        }
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct UnknownConfigWrapper {
    #[serde(flatten)]
    inner: toml::Value,
}

impl UnknownConfigWrapper {
    fn find_value(&self, key: &str) -> Option<&toml::Value> {
        find_toml_value(&self.inner, key)
    }

    pub(crate) fn try_get_id(&self) -> anyhow::Result<&str> {
        let id_val = self
            .find_value("id")
            .ok_or_else(|| anyhow!("no id field present in the config"))?;
        if let toml::Value::String(id) = id_val {
            Ok(id)
        } else {
            bail!("no id field present in the config")
        }
    }

    pub(crate) fn try_get_credentials_store(&self) -> anyhow::Result<PathBuf> {
        let id_val = self
            .find_value("credentials_database_path")
            .ok_or_else(|| anyhow!("no 'credentials_database_path' field present in the config"))?;
        if let toml::Value::String(credentials_store) = id_val {
            Ok(credentials_store.parse()?)
        } else {
            bail!("no 'credentials_database_path' field present in the config")
        }
    }

    pub(crate) fn try_get_private_id_key(&self) -> anyhow::Result<PathBuf> {
        let id_val = self
            .find_value("keys.private_identity_key_file")
            .ok_or_else(|| {
                anyhow!("no 'keys.private_identity_key_file' field present in the config")
            })?;
        if let toml::Value::String(pub_id_key) = id_val {
            Ok(pub_id_key.parse()?)
        } else {
            bail!("no 'keys.private_identity_key_file' field present in the config")
        }
    }
}
