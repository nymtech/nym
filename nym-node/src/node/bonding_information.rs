// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymNodeError;
use crate::node::helpers::load_ed25519_identity_public_key;
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Deserialize, Serialize)]
pub struct BondingInformation {
    host: String,
    identity_key: ed25519::PublicKey,
}

impl Display for BondingInformation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Identity Key: {}", self.identity_key)?;
        writeln!(f, "Host: {}", self.host)?;
        writeln!(f, "Custom HTTP Port: you might want to set it if your node won't be accessible on any of the ports: 80/443/8080")?;

        Ok(())
    }
}

impl BondingInformation {
    pub fn from_data(config: &Config, ed25519_identity_key: ed25519::PublicKey) -> Self {
        let host = match config.host.hostname {
            Some(ref host) => host.clone(),
            None => match config.host.public_ips.first() {
                Some(first_ip) => {
                    if !first_ip.is_loopback()
                        && !first_ip.is_multicast()
                        && !first_ip.is_unspecified()
                    {
                        first_ip.to_string()
                    } else {
                        "NO KNOWN VALID HOSTNAMES - YOU NEED TO FILL IT MANUALLY".to_string()
                    }
                }
                None => "NO KNOWN VALID HOSTNAMES - YOU NEED TO FILL IT MANUALLY".to_string(),
            },
        };

        BondingInformation {
            host,
            identity_key: ed25519_identity_key,
        }
    }

    pub fn try_load(config: &Config) -> Result<BondingInformation, NymNodeError> {
        let ed25519_identity_key = load_ed25519_identity_public_key(
            &config.storage_paths.keys.public_ed25519_identity_key_file,
        )?;

        Ok(Self::from_data(config, ed25519_identity_key))
    }
}
