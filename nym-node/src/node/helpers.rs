// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::{ed25519, x25519};
use nym_node::config::NodeMode;
use nym_node::error::{KeyIOFailure, NymNodeError};
use nym_node_http_api::api::api_requests::v1::node::models::NodeDescription;
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use nym_pemstore::KeyPairPath;
use semver::{BuildMetadata, Version};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::path::Path;

#[allow(clippy::unwrap_used)]
pub fn bonding_version() -> String {
    // SAFETY:
    // the value has been put there by cargo
    let raw = env!("CARGO_PKG_VERSION");
    let mut semver: Version = raw.parse().unwrap();

    // if it's not empty, then we messed up our own versioning
    assert!(semver.build.is_empty());
    semver.build = BuildMetadata::new("nymnode").unwrap();
    semver.to_string()
}

#[derive(Debug, Serialize)]
pub(crate) struct DisplayDetails {
    pub(crate) current_mode: NodeMode,

    pub(crate) description: NodeDescription,

    pub(crate) ed25519_identity_key: String,
    pub(crate) x25519_sphinx_key: String,
    pub(crate) x25519_noise_key: String,
    pub(crate) x25519_wireguard_key: String,

    pub(crate) exit_network_requester_address: String,
    pub(crate) exit_ip_packet_router_address: String,
}

impl Display for DisplayDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "current mode: {}", self.current_mode)?;
        writeln!(f, "moniker: '{}'", self.description.moniker)?;
        writeln!(f, "website: '{}'", self.description.website)?;
        writeln!(
            f,
            "security contact: '{}'",
            self.description.security_contact
        )?;
        writeln!(f, "details: '{}'", self.description.details)?;
        writeln!(f, "ed25519 identity: {}", self.ed25519_identity_key)?;
        writeln!(f, "x25519 sphinx: {}", self.x25519_sphinx_key)?;
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
        Ok(())
    }
}

pub(crate) fn load_keypair<T: PemStorableKeyPair>(
    paths: KeyPairPath,
    name: impl Into<String>,
) -> Result<T, KeyIOFailure> {
    nym_pemstore::load_keypair(&paths).map_err(|err| KeyIOFailure::KeyPairLoadFailure {
        keys: name.into(),
        paths,
        err,
    })
}

pub(crate) fn store_keypair<T: PemStorableKeyPair>(
    keys: &T,
    paths: KeyPairPath,
    name: impl Into<String>,
) -> Result<(), KeyIOFailure> {
    nym_pemstore::store_keypair(keys, &paths).map_err(|err| KeyIOFailure::KeyPairStoreFailure {
        keys: name.into(),
        paths,
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
    paths: KeyPairPath,
) -> Result<ed25519::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "ed25519-identity")?)
}

#[allow(dead_code)]
pub(crate) fn load_ed25519_identity_public_key<P: AsRef<Path>>(
    path: P,
) -> Result<ed25519::PublicKey, NymNodeError> {
    Ok(load_key(path, "ed25519-identity-public-key")?)
}

pub(crate) fn load_x25519_sphinx_keypair(
    paths: KeyPairPath,
) -> Result<x25519::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "x25519-sphinx")?)
}

pub(crate) fn load_x25519_noise_keypair(
    paths: KeyPairPath,
) -> Result<x25519::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "x25519-noise")?)
}

pub(crate) fn load_x25519_wireguard_keypair(
    paths: KeyPairPath,
) -> Result<x25519::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "x25519-wireguard")?)
}

pub(crate) fn load_x25519_sphinx_public_key<P: AsRef<Path>>(
    path: P,
) -> Result<x25519::PublicKey, NymNodeError> {
    Ok(load_key(path, "x25519-sphinx-public-key")?)
}

pub(crate) fn store_ed25519_identity_keypair(
    keys: &ed25519::KeyPair,
    paths: KeyPairPath,
) -> Result<(), NymNodeError> {
    Ok(store_keypair(keys, paths, "ed25519-identity")?)
}

pub(crate) fn store_x25519_sphinx_keypair(
    keys: &x25519::KeyPair,
    paths: KeyPairPath,
) -> Result<(), NymNodeError> {
    Ok(store_keypair(keys, paths, "x25519-sphinx")?)
}

pub(crate) fn store_x25519_noise_keypair(
    keys: &x25519::KeyPair,
    paths: KeyPairPath,
) -> Result<(), NymNodeError> {
    Ok(store_keypair(keys, paths, "x25519-noise")?)
}

pub(crate) fn store_x25519_wireguard_keypair(
    keys: &x25519::KeyPair,
    paths: KeyPairPath,
) -> Result<(), NymNodeError> {
    Ok(store_keypair(keys, paths, "x25519-wireguard")?)
}
