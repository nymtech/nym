// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::helpers::{load_ed25519_identity_public_key, load_x25519_sphinx_public_key};
use nym_crypto::asymmetric::{encryption, identity};
use nym_node::config::Config;
use nym_node::error::NymNodeError;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

// TODO: work in progress, I'm not 100% sure yet what will be needed
#[derive(Serialize, Deserialize, Debug)]
pub struct BondingInformationV1 {
    pub(crate) ed25519_identity_key: identity::PublicKey,
    pub(crate) x25519_sphinx_key: encryption::PublicKey,
}

impl Display for BondingInformationV1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "ed25519 identity key: {}",
            self.ed25519_identity_key.to_base58_string()
        )?;
        write!(
            f,
            "x25519 sphinx key: {}",
            self.x25519_sphinx_key.to_base58_string()
        )
    }
}

impl BondingInformationV1 {
    pub fn from_data(
        ed25519_identity_key: &identity::PublicKey,
        x25519_sphinx_key: &encryption::PublicKey,
    ) -> BondingInformationV1 {
        BondingInformationV1 {
            ed25519_identity_key: *ed25519_identity_key,
            x25519_sphinx_key: *x25519_sphinx_key,
        }
    }

    pub fn try_load(config: &Config) -> Result<BondingInformationV1, NymNodeError> {
        Ok(BondingInformationV1 {
            ed25519_identity_key: load_ed25519_identity_public_key(
                &config.storage_paths.keys.public_ed25519_identity_key_file,
            )?,
            x25519_sphinx_key: load_x25519_sphinx_public_key(
                &config.storage_paths.keys.public_x25519_sphinx_key_file,
            )?,
        })
    }
}
