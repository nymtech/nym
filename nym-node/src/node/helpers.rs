// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::{encryption, identity};
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
) -> Result<identity::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "ed25519-identity")?)
}

pub(crate) fn load_ed25519_identity_public_key<P: AsRef<Path>>(
    path: P,
) -> Result<identity::PublicKey, NymNodeError> {
    Ok(load_key(path, "ed25519-identity-public-key")?)
}

pub(crate) fn load_x25519_sphinx_keypair(
    paths: KeyPairPath,
) -> Result<encryption::KeyPair, NymNodeError> {
    Ok(load_keypair(paths, "x25519-sphinx")?)
}

pub(crate) fn load_x25519_sphinx_public_key<P: AsRef<Path>>(
    path: P,
) -> Result<encryption::PublicKey, NymNodeError> {
    Ok(load_key(path, "x25519-sphinx-public-key")?)
}

pub(crate) fn store_ed25519_identity_keypair(
    keys: &identity::KeyPair,
    paths: KeyPairPath,
) -> Result<(), NymNodeError> {
    Ok(store_keypair(keys, paths, "ed25519-identity")?)
}

pub(crate) fn store_x25519_sphinx_keypair(
    keys: &encryption::KeyPair,
    paths: KeyPairPath,
) -> Result<(), NymNodeError> {
    Ok(store_keypair(keys, paths, "x25519-sphinx")?)
}
