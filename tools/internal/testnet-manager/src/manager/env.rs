// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::manager::network::LoadedNetwork;
use nym_config::defaults::var_names;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tracing::{trace, warn};

#[derive(Default)]
pub struct Env {
    pub(crate) mixnet_contract_address: Option<String>,
    pub(crate) vesting_contract_address: Option<String>,
    pub(crate) ecash_contract_address: Option<String>,
    pub(crate) cw4_group_contract_address: Option<String>,
    pub(crate) cw3_multisig_contract_address: Option<String>,
    pub(crate) dkg_contract_address: Option<String>,
    pub(crate) nyxd_endpoint: Option<String>,
    pub(crate) nym_api_endpoint: Option<String>,
}

impl Env {
    pub fn with_nym_api<S: Into<String>>(mut self, nym_api: S) -> Self {
        self.nym_api_endpoint = Some(nym_api.into());
        self
    }

    // this will be used eventually
    #[allow(dead_code)]
    pub fn try_load<P: AsRef<Path>>(path: P) -> Result<Self, NetworkManagerError> {
        let mut env = Env::default();
        let content = fs::read_to_string(path)?;

        for entry in content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
            let Some((k, v)) = entry.split_once('=') else {
                warn!("malformed .env entry: '{entry}'");
                continue;
            };

            match k {
                var_names::CONFIGURED
                | var_names::BECH32_PREFIX
                | var_names::MIX_DENOM
                | var_names::MIX_DENOM_DISPLAY
                | var_names::STAKE_DENOM
                | var_names::STAKE_DENOM_DISPLAY
                | var_names::DENOMS_EXPONENT => {
                    trace!("ignoring values for {k} and using default instead")
                }
                var_names::MIXNET_CONTRACT_ADDRESS => {
                    env.mixnet_contract_address = Some(v.to_string())
                }
                var_names::VESTING_CONTRACT_ADDRESS => {
                    env.vesting_contract_address = Some(v.to_string())
                }
                var_names::ECASH_CONTRACT_ADDRESS => {
                    env.ecash_contract_address = Some(v.to_string())
                }
                var_names::GROUP_CONTRACT_ADDRESS => {
                    env.cw4_group_contract_address = Some(v.to_string())
                }
                var_names::MULTISIG_CONTRACT_ADDRESS => {
                    env.cw3_multisig_contract_address = Some(v.to_string())
                }
                var_names::COCONUT_DKG_CONTRACT_ADDRESS => {
                    env.dkg_contract_address = Some(v.to_string())
                }
                var_names::NYXD => env.nyxd_endpoint = Some(v.to_string()),
                var_names::NYM_API => env.nym_api_endpoint = Some(v.to_string()),
                other => warn!("unsupported .env entry: '{other}'"),
            }
        }

        Ok(env)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), NetworkManagerError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut env_file = File::create(path)?;
        let content = self.to_string();
        env_file.write_all(content.as_bytes())?;
        Ok(())
    }
}

impl<'a> From<&'a LoadedNetwork> for Env {
    fn from(network: &'a LoadedNetwork) -> Self {
        Env {
            mixnet_contract_address: Some(network.contracts.mixnet.address.to_string()),
            vesting_contract_address: Some(network.contracts.vesting.address.to_string()),
            ecash_contract_address: Some(network.contracts.ecash.address.to_string()),
            cw4_group_contract_address: Some(network.contracts.cw4_group.address.to_string()),
            cw3_multisig_contract_address: Some(network.contracts.cw3_multisig.address.to_string()),
            dkg_contract_address: Some(network.contracts.dkg.address.to_string()),
            nyxd_endpoint: Some(network.rpc_endpoint.to_string()),
            nym_api_endpoint: None,
        }
    }
}

impl Display for Env {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CONFIGURED=true\n\
\n\
BECH32_PREFIX=n\n\
MIX_DENOM=unym\n\
MIX_DENOM_DISPLAY=nym\n\
STAKE_DENOM=unyx\n\
STAKE_DENOM_DISPLAY=nyx\n\
DENOMS_EXPONENT=6\n\
\n\
"
        )?;
        if let Some(mixnet_contract_address) = &self.mixnet_contract_address {
            writeln!(
                f,
                "{}={mixnet_contract_address}",
                var_names::MIXNET_CONTRACT_ADDRESS
            )?;
        }
        if let Some(vesting_contract_address) = &self.vesting_contract_address {
            writeln!(
                f,
                "{}={vesting_contract_address}",
                var_names::VESTING_CONTRACT_ADDRESS
            )?;
        }
        if let Some(ecash_contract_address) = &self.ecash_contract_address {
            writeln!(
                f,
                "{}={ecash_contract_address}",
                var_names::ECASH_CONTRACT_ADDRESS
            )?;
        }
        if let Some(cw4_group_contract_address) = &self.cw4_group_contract_address {
            writeln!(
                f,
                "{}={cw4_group_contract_address}",
                var_names::GROUP_CONTRACT_ADDRESS
            )?;
        }
        if let Some(cw3_multisig_contract_address) = &self.cw3_multisig_contract_address {
            writeln!(
                f,
                "{}={cw3_multisig_contract_address}",
                var_names::MULTISIG_CONTRACT_ADDRESS
            )?;
        }
        if let Some(dkg_contract_address) = &self.dkg_contract_address {
            writeln!(
                f,
                "{}={dkg_contract_address}",
                var_names::COCONUT_DKG_CONTRACT_ADDRESS
            )?;
        }
        if let Some(nyxd_endpoint) = &self.nyxd_endpoint {
            writeln!(f, "{}={nyxd_endpoint}", var_names::NYXD)?;
        }
        if let Some(nym_api_endpoint) = &self.nym_api_endpoint {
            writeln!(f, "{}={nym_api_endpoint}", var_names::NYM_API)?;
        }
        Ok(())
    }
}
