// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use crate::manager::contract::{Account, LoadedNymContracts, NymContracts};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    pub name: String,

    pub rpc_endpoint: Url,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    pub contracts: NymContracts,

    pub auxiliary_addresses: SpecialAddresses,
}

impl Network {
    pub fn unchecked_to_env_file_section(&self) -> String {
        format!(
            "\
\n\
\n\
REWARDING_VALIDATOR_ADDRESS={}\n\
MIXNET_CONTRACT_ADDRESS={}\n\
VESTING_CONTRACT_ADDRESS={}\n\
ECASH_CONTRACT_ADDRESS={}\n\
GROUP_CONTRACT_ADDRESS={}\n\
MULTISIG_CONTRACT_ADDRESS={}\n\
COCONUT_DKG_CONTRACT_ADDRESS={}\n\
NYXD={}\n\
",
            self.auxiliary_addresses.mixnet_rewarder.address,
            self.contracts.mixnet.address().unwrap(),
            self.contracts.vesting.address().unwrap(),
            self.contracts.ecash.address().unwrap(),
            self.contracts.cw4_group.address().unwrap(),
            self.contracts.cw3_multisig.address().unwrap(),
            self.contracts.dkg.address().unwrap(),
            self.rpc_endpoint,
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadedNetwork {
    pub name: String,

    pub rpc_endpoint: Url,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    pub contracts: LoadedNymContracts,

    pub auxiliary_addresses: SpecialAddresses,
}

impl LoadedNetwork {
    pub fn to_env_file_section(&self) -> String {
        format!(
            "\
\n\
\n\
REWARDING_VALIDATOR_ADDRESS={}\n\
MIXNET_CONTRACT_ADDRESS={}\n\
VESTING_CONTRACT_ADDRESS={}\n\
ECASH_CONTRACT_ADDRESS={}\n\
GROUP_CONTRACT_ADDRESS={}\n\
MULTISIG_CONTRACT_ADDRESS={}\n\
COCONUT_DKG_CONTRACT_ADDRESS={}\n\
NYXD={}\n\
",
            self.auxiliary_addresses.mixnet_rewarder.address,
            self.contracts.mixnet.address,
            self.contracts.vesting.address,
            self.contracts.ecash.address,
            self.contracts.cw4_group.address,
            self.contracts.cw3_multisig.address,
            self.contracts.dkg.address,
            self.rpc_endpoint,
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpecialAddresses {
    pub ecash_holding_account: Account,
    pub mixnet_rewarder: Account,
}

impl Default for SpecialAddresses {
    fn default() -> Self {
        SpecialAddresses {
            ecash_holding_account: Account::new(),
            mixnet_rewarder: Account::new(),
        }
    }
}
