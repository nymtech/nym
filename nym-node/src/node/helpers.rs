// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::{ed25519, x25519};
use nym_node::error::{KeyIOFailure, NymNodeError};
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use nym_pemstore::KeyPairPath;
use std::path::Path;

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
