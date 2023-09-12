// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::GatewayError;
use crate::node::storage::PersistentStorage;
use nym_crypto::asymmetric::{encryption, identity};
use nym_pemstore::traits::PemStorableKeyPair;
use nym_pemstore::KeyPairPath;
use std::path::Path;

pub(crate) fn load_network_requester_config<P: AsRef<Path>>(
    id: &str,
    path: P,
) -> Result<nym_network_requester::Config, GatewayError> {
    let path = path.as_ref();
    nym_network_requester::Config::read_from_toml_file(path).map_err(|err| {
        GatewayError::NetworkRequesterConfigLoadFailure {
            id: id.to_string(),
            path: path.to_path_buf(),
            source: err,
        }
    })
}

pub(crate) async fn initialise_main_storage(
    config: &Config,
) -> Result<PersistentStorage, GatewayError> {
    let path = &config.storage_paths.clients_storage;
    let retrieval_limit = config.debug.message_retrieval_limit;

    Ok(PersistentStorage::init(path, retrieval_limit).await?)
}

pub(crate) fn load_keypair<T: PemStorableKeyPair>(
    paths: KeyPairPath,
    name: impl Into<String>,
) -> Result<T, GatewayError> {
    nym_pemstore::load_keypair(&paths).map_err(|err| GatewayError::KeyPairLoadFailure {
        keys: name.into(),
        paths,
        err,
    })
}

/// Loads identity keys stored on disk
pub(crate) fn load_identity_keys(config: &Config) -> Result<identity::KeyPair, GatewayError> {
    let identity_paths = KeyPairPath::new(
        config.storage_paths.keys.private_identity_key(),
        config.storage_paths.keys.public_identity_key(),
    );
    load_keypair(identity_paths, "gateway identity")
}

/// Loads Sphinx keys stored on disk
pub(crate) fn load_sphinx_keys(config: &Config) -> Result<encryption::KeyPair, GatewayError> {
    let sphinx_paths = KeyPairPath::new(
        config.storage_paths.keys.private_encryption_key(),
        config.storage_paths.keys.public_encryption_key(),
    );
    load_keypair(sphinx_paths, "gateway sphinx")
}
