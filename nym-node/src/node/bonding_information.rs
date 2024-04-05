// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::helpers::{load_ed25519_identity_public_key, load_x25519_sphinx_public_key};
use nym_node::config::{Config, NodeMode};
use nym_node::error::NymNodeError;
use semver::{BuildMetadata, Version};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "node_type")]
pub enum BondingInformationV1 {
    Mixnode(MixnodeBondingInformation),
    Gateway(GatewayBondingInformation),
}

impl Display for BondingInformationV1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BondingInformationV1::Mixnode(m) => m.fmt(f),
            BondingInformationV1::Gateway(g) => g.fmt(f),
        }
    }
}

// TODO: work in progress, I'm not 100% sure yet what will be needed
// #[derive(Serialize, Deserialize, Debug)]
// pub struct BondingInformationV2 {
//     pub(crate) ed25519_identity_key: ed25519::PublicKey,
//     pub(crate) x25519_sphinx_key: x25519::PublicKey,
// }
//
// impl Display for BondingInformationV2 {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         writeln!(
//             f,
//             "ed25519 identity key: {}",
//             self.ed25519_identity_key.to_base58_string()
//         )?;
//         write!(
//             f,
//             "x25519 sphinx key: {}",
//             self.x25519_sphinx_key.to_base58_string()
//         )
//     }
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct MixnodeBondingInformation {
    pub(crate) version: String,
    pub(crate) host: String,
    pub(crate) identity_key: String,
    pub(crate) sphinx_key: String,
}

impl MixnodeBondingInformation {
    pub fn from_data(
        ed25519_identity_key: String,
        x25519_sphinx_key: String,
    ) -> MixnodeBondingInformation {
        MixnodeBondingInformation {
            version: Self::get_version(),
            host: "YOU NEED TO FILL THIS FIELD MANUALLY".to_string(),
            identity_key: ed25519_identity_key,
            sphinx_key: x25519_sphinx_key,
        }
    }

    #[allow(clippy::unwrap_used)]
    fn get_version() -> String {
        // SAFETY:
        // 1. the value has been put into the environment during build.rs, so it must exist,
        // 2. and the obtained version has already been parsed into semver in build.rs, so it must be a valid semver
        let raw = include_str!(concat!(env!("OUT_DIR"), "/mixnode_version"));
        let mut semver: Version = raw.parse().unwrap();

        // if it's not empty, then we messed up our own versioning
        assert!(semver.build.is_empty());
        semver.build = BuildMetadata::new("nymnode").unwrap();
        semver.to_string()
    }
}

impl Display for MixnodeBondingInformation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Node type: Mixnode")?;
        writeln!(f, "Identity Key: {}", self.identity_key)?;
        writeln!(f, "Sphinx Key: {}", self.sphinx_key)?;
        writeln!(f, "Host: {}", self.host)?;
        writeln!(f, "Version: {}", self.version)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GatewayBondingInformation {
    pub(crate) version: String,
    pub(crate) host: String,
    pub(crate) location: String,
    pub(crate) identity_key: String,
    pub(crate) sphinx_key: String,
}

impl GatewayBondingInformation {
    pub fn from_data(
        ed25519_identity_key: String,
        x25519_sphinx_key: String,
    ) -> GatewayBondingInformation {
        GatewayBondingInformation {
            version: Self::get_version(),
            host: "YOU NEED TO FILL THIS FIELD MANUALLY".to_string(),
            location: "YOU NEED TO FILL THIS FIELD MANUALLY".to_string(),
            identity_key: ed25519_identity_key,
            sphinx_key: x25519_sphinx_key,
        }
    }

    #[allow(clippy::unwrap_used)]
    fn get_version() -> String {
        // SAFETY:
        // 1. the value has been put into the file during build.rs, so it must exist,
        // 2. and the obtained version has already been parsed into semver in build.rs, so it must be a valid semver
        let raw = include_str!(concat!(env!("OUT_DIR"), "/gateway_version"));
        let mut semver: Version = raw.parse().unwrap();

        // if it's not empty, then we messed up our own versioning
        assert!(semver.build.is_empty());
        semver.build = BuildMetadata::new("nymnode").unwrap();
        semver.to_string()
    }
}

impl Display for GatewayBondingInformation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Node type: Gateway")?;
        writeln!(f, "Identity Key: {}", self.identity_key)?;
        writeln!(f, "Sphinx Key: {}", self.sphinx_key)?;
        writeln!(f, "Location: {}", self.location)?;
        writeln!(f, "Host: {}", self.host)?;
        writeln!(f, "Version: {}", self.version)?;
        Ok(())
    }
}

impl BondingInformationV1 {
    pub fn from_data(
        mode: NodeMode,
        ed25519_identity_key: String,
        x25519_sphinx_key: String,
    ) -> BondingInformationV1 {
        match mode {
            NodeMode::Mixnode => BondingInformationV1::Mixnode(
                MixnodeBondingInformation::from_data(ed25519_identity_key, x25519_sphinx_key),
            ),
            NodeMode::EntryGateway | NodeMode::ExitGateway => BondingInformationV1::Gateway(
                GatewayBondingInformation::from_data(ed25519_identity_key, x25519_sphinx_key),
            ),
        }
    }

    fn ed25519_identity_key(&self) -> String {
        match self {
            BondingInformationV1::Mixnode(m) => m.identity_key.clone(),
            BondingInformationV1::Gateway(g) => g.identity_key.clone(),
        }
    }

    fn x25519_sphinx_key(&self) -> String {
        match self {
            BondingInformationV1::Mixnode(m) => m.sphinx_key.clone(),
            BondingInformationV1::Gateway(g) => g.sphinx_key.clone(),
        }
    }

    pub fn try_load(config: &Config) -> Result<BondingInformationV1, NymNodeError> {
        let ed25519_identity_key = load_ed25519_identity_public_key(
            &config.storage_paths.keys.public_ed25519_identity_key_file,
        )?;
        let x25519_sphinx_key = load_x25519_sphinx_public_key(
            &config.storage_paths.keys.public_x25519_sphinx_key_file,
        )?;
        let mode = config.mode;

        Ok(Self::from_data(
            mode,
            ed25519_identity_key.to_base58_string(),
            x25519_sphinx_key.to_base58_string(),
        ))
    }

    pub fn with_mode(self, mode: NodeMode) -> Self {
        let identity_key = self.ed25519_identity_key();
        let sphinx_key = self.x25519_sphinx_key();
        Self::from_data(mode, identity_key, sphinx_key)
    }
}
