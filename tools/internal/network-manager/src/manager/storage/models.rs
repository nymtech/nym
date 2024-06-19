// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NetworkManagerError;
use crate::manager::contract::{Account, LoadedContract};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(FromRow)]
pub(crate) struct RawAccount {
    pub(crate) address: String,
    pub(crate) mnemonic: String,
}

impl TryFrom<RawAccount> for Account {
    type Error = NetworkManagerError;

    fn try_from(value: RawAccount) -> Result<Self, Self::Error> {
        Ok(Account {
            address: value
                .address
                .parse()
                .map_err(|_| NetworkManagerError::MalformedAccountAddress)?,
            mnemonic: value.mnemonic.parse()?,
        })
    }
}

#[derive(FromRow)]
pub(crate) struct RawContract {
    #[allow(unused)]
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) address: String,
    pub(crate) admin_address: String,
    pub(crate) mnemonic: String,
}

impl TryFrom<RawContract> for LoadedContract {
    type Error = NetworkManagerError;

    fn try_from(value: RawContract) -> Result<Self, Self::Error> {
        Ok(LoadedContract {
            name: value.name,
            address: value
                .address
                .parse()
                .map_err(|_| NetworkManagerError::MalformedAccountAddress)?,
            admin_address: value
                .admin_address
                .parse()
                .map_err(|_| NetworkManagerError::MalformedAccountAddress)?,
            admin_mnemonic: value
                .mnemonic
                .parse()
                .map_err(|_| NetworkManagerError::MalformedAccountAddress)?,
        })
    }
}

#[derive(FromRow)]
pub(crate) struct RawNetwork {
    #[allow(unused)]
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) created_at: OffsetDateTime,

    pub(crate) mixnet_contract_id: i64,
    pub(crate) vesting_contract_id: i64,
    pub(crate) ecash_contract_id: i64,
    pub(crate) cw3_multisig_contract_id: i64,
    pub(crate) cw4_group_contract_id: i64,
    pub(crate) dkg_contract_id: i64,

    pub(crate) rewarder_address: String,
    pub(crate) ecash_holding_account_address: String,
}
