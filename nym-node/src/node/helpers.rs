// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::NodeModes;
use crate::error::{KeyIOFailure, NymNodeError};
use crate::node::key_rotation::key::{SphinxPrivateKey, SphinxPublicKey};
use crate::node::nym_apis_client::NymApisClient;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_node_requests::api::v1::node::models::NodeDescription;
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use nym_pemstore::KeyPairPath;
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::QueryHttpRpcNyxdClient;
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::path::Path;
use tracing::warn;
use url::Url;

#[derive(Debug, Serialize)]
pub(crate) struct DisplaySphinxKey {
    public_key: String,
    rotation_id: u32,
}

impl From<&SphinxPrivateKey> for DisplaySphinxKey {
    fn from(value: &SphinxPrivateKey) -> Self {
        let pubkey: SphinxPublicKey = value.into();
        DisplaySphinxKey {
            public_key: pubkey.inner.to_base58_string(),
            rotation_id: pubkey.rotation_id,
        }
    }
}

impl Display for DisplaySphinxKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (rotation: {})", self.public_key, self.rotation_id)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct DisplayDetails {
    pub(crate) current_modes: NodeModes,

    pub(crate) description: NodeDescription,

    pub(crate) ed25519_identity_key: String,
    pub(crate) x25519_primary_sphinx_key: DisplaySphinxKey,
    pub(crate) x25519_secondary_sphinx_key: Option<DisplaySphinxKey>,
    pub(crate) x25519_noise_key: String,
    pub(crate) x25519_wireguard_key: String,

    pub(crate) exit_network_requester_address: String,
    pub(crate) exit_ip_packet_router_address: String,
    pub(crate) exit_authenticator_address: String,
}

impl Display for DisplayDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "current mode: {:#?}", self.current_modes)?;
        writeln!(f, "moniker: '{}'", self.description.moniker)?;
        writeln!(f, "website: '{}'", self.description.website)?;
        writeln!(
            f,
            "security contact: '{}'",
            self.description.security_contact
        )?;
        writeln!(f, "details: '{}'", self.description.details)?;
        writeln!(f, "ed25519 identity: {}", self.ed25519_identity_key)?;
        writeln!(
            f,
            "x25519 primary sphinx: {}",
            self.x25519_primary_sphinx_key
        )?;
        if let Some(secondary) = &self.x25519_secondary_sphinx_key {
            writeln!(f, "x25519 primary sphinx: {secondary}")?;
        }
        writeln!(f, "x25519 noise: {}", self.x25519_noise_key)?;
        writeln!(
            f,
            "exit network requester address: {}",
            self.exit_network_requester_address
        )?;
        writeln!(
            f,
            "exit ip packet router address: {}",
            self.exit_ip_packet_router_address
        )?;
        writeln!(
            f,
            "exit authenticator address: {}",
            self.exit_authenticator_address
        )?;
        Ok(())
    }
}

pub(crate) fn load_keypair<T: PemStorableKeyPair>(
    paths: &KeyPairPath,
    name: impl Into<String>,
) -> Result<T, KeyIOFailure> {
    nym_pemstore::load_keypair(paths).map_err(|err| KeyIOFailure::KeyPairLoadFailure {
        keys: name.into(),
        paths: paths.clone(),
        err,
    })
}

pub(crate) fn store_keypair<T: PemStorableKeyPair>(
    keys: &T,
    paths: &KeyPairPath,
    name: impl Into<String>,
) -> Result<(), KeyIOFailure> {
    nym_pemstore::store_keypair(keys, paths).map_err(|err| KeyIOFailure::KeyPairStoreFailure {
        keys: name.into(),
        paths: paths.clone(),
        err,
    })
}

pub(crate) fn load_key<T, P>(path: P, name: impl Into<String>) -> Result<T, KeyIOFailure>
where
    T: PemStorableKey,
    P: AsRef<Path>,
{
    nym_pemstore::load_key(path.as_ref()).map_err(|err| KeyIOFailure::KeyLoadFailure {
        key: name.into(),
        path: path.as_ref().to_path_buf(),
        err,
    })
}

pub(crate) fn store_key<T, P>(key: &T, path: P, name: impl Into<String>) -> Result<(), KeyIOFailure>
where
    T: PemStorableKey,
    P: AsRef<Path>,
{
    nym_pemstore::store_key(key, path.as_ref()).map_err(|err| KeyIOFailure::KeyStoreFailure {
        key: name.into(),
        path: path.as_ref().to_path_buf(),
        err,
    })
}

pub(crate) fn load_ed25519_identity_keypair(
    paths: &KeyPairPath,
) -> Result<ed25519::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "ed25519-identity")?)
}

#[allow(dead_code)]
pub(crate) fn load_ed25519_identity_public_key<P: AsRef<Path>>(
    path: P,
) -> Result<ed25519::PublicKey, NymNodeError> {
    Ok(load_key(path, "ed25519-identity-public-key")?)
}

pub(crate) fn load_x25519_noise_keypair(
    paths: &KeyPairPath,
) -> Result<x25519::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "x25519-noise")?)
}

pub(crate) fn load_x25519_wireguard_keypair(
    paths: &KeyPairPath,
) -> Result<x25519::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "x25519-wireguard")?)
}

pub(crate) fn store_ed25519_identity_keypair(
    keys: &ed25519::KeyPair,
    paths: &KeyPairPath,
) -> Result<(), NymNodeError> {
    Ok(store_keypair(keys, paths, "ed25519-identity")?)
}

pub(crate) fn store_x25519_noise_keypair(
    keys: &x25519::KeyPair,
    paths: &KeyPairPath,
) -> Result<(), NymNodeError> {
    Ok(store_keypair(keys, paths, "x25519-noise")?)
}

pub(crate) async fn get_current_rotation_id(
    nym_apis: &[Url],
    fallback_nyxd: &[Url],
) -> Result<u32, NymNodeError> {
    let apis_client = NymApisClient::new(nym_apis, ShutdownToken::ephemeral())?;
    if let Ok(rotation_info) = apis_client.get_key_rotation_info().await.map(|r| r.details) {
        if rotation_info.is_epoch_stuck() {
            return Err(NymNodeError::StuckEpoch);
        }
        let current_epoch = rotation_info.current_absolute_epoch_id;
        return Ok(rotation_info
            .key_rotation_state
            .key_rotation_id(current_epoch));
    }
    warn!("failed to retrieve key rotation id from nym apis. falling back to contract query");

    for nyxd_url in fallback_nyxd {
        let client = QueryHttpRpcNyxdClient::connect_to_default_env(nyxd_url.as_str())?;
        if let Ok(res) = client.get_key_rotation_id().await {
            return Ok(res.rotation_id);
        }
    }

    Err(NymNodeError::NymApisExhausted)
}
