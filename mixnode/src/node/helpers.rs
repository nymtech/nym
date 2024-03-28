// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::MixnodeError;
use nym_crypto::asymmetric::{encryption, identity};
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use nym_pemstore::KeyPairPath;
use std::path::Path;

pub(crate) fn load_keypair<T: PemStorableKeyPair>(
    paths: KeyPairPath,
    name: impl Into<String>,
) -> Result<T, MixnodeError> {
    nym_pemstore::load_keypair(&paths).map_err(|err| MixnodeError::KeyPairLoadFailure {
        keys: name.into(),
        paths,
        err,
    })
}

#[allow(unused)]
pub(crate) fn load_public_key<T, P, S>(path: P, name: S) -> Result<T, MixnodeError>
where
    T: PemStorableKey,
    P: AsRef<Path>,
    S: Into<String>,
{
    nym_pemstore::load_key(path.as_ref()).map_err(|err| MixnodeError::PublicKeyLoadFailure {
        key: name.into(),
        path: path.as_ref().to_path_buf(),
        err,
    })
}

/// Loads identity keys stored on disk
pub fn load_identity_keys(config: &Config) -> Result<identity::KeyPair, MixnodeError> {
    let identity_paths = KeyPairPath::new(
        config.storage_paths.keys.private_identity_key(),
        config.storage_paths.keys.public_identity_key(),
    );
    load_keypair(identity_paths, "mixnode identity")
}

/// Loads Sphinx keys stored on disk
pub fn load_sphinx_keys(config: &Config) -> Result<encryption::KeyPair, MixnodeError> {
    let sphinx_paths = KeyPairPath::new(
        config.storage_paths.keys.private_encryption_key(),
        config.storage_paths.keys.public_encryption_key(),
    );
    load_keypair(sphinx_paths, "mixnode sphinx")
}
