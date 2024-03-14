// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MixnodeError {
    #[error("failed to load {keys} keys from '{}' (private key) and '{}' (public key): {err}", .paths.private_key_path.display(), .paths.public_key_path.display())]
    KeyPairLoadFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: io::Error,
    },

    #[allow(dead_code)]
    #[error("failed to load {key} public key from '{}': {err}", .path.display())]
    PublicKeyLoadFailure {
        key: String,
        path: PathBuf,
        #[source]
        err: io::Error,
    },

    #[error(
    "failed to load config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigLoadFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigSaveFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    // TODO: in the future this should work the other way, i.e. NymNode depending on Gateway errors
    #[error(transparent)]
    NymNodeHttpError(#[from] nym_node_http_api::NymNodeHttpError),
}
