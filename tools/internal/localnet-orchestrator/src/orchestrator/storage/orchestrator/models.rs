// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::orchestrator::account::Account;
use anyhow::Context;
use sqlx::FromRow;
use time::OffsetDateTime;

#[allow(dead_code)]
#[derive(FromRow)]
pub(crate) struct RawLocalnetContracts {
    pub(crate) metadata_id: i64,
    pub(crate) mixnet_contract_id: i64,
    pub(crate) vesting_contract_id: i64,
    pub(crate) ecash_contract_id: i64,
    pub(crate) cw3_multisig_contract_id: i64,
    pub(crate) cw4_group_contract_id: i64,
    pub(crate) dkg_contract_id: i64,
    pub(crate) performance_contract_id: i64,
}

#[allow(dead_code)]
#[derive(FromRow)]
pub(crate) struct RawAuthorisedNetworkMonitor {
    pub(crate) network_id: i64,
    pub(crate) address: String,
}

#[allow(dead_code)]
#[derive(FromRow)]
pub(crate) struct RawAuxiliaryAccounts {
    pub(crate) network_id: i64,
    pub(crate) rewarder_address: String,
    pub(crate) ecash_holding_account_address: String,
}

#[derive(FromRow)]
pub(crate) struct RawAccount {
    pub(crate) address: String,
    pub(crate) mnemonic: String,
}

impl TryFrom<RawAccount> for Account {
    type Error = anyhow::Error;

    fn try_from(value: RawAccount) -> Result<Self, Self::Error> {
        Ok(Account {
            address: value
                .address
                .parse()
                .map_err(|err| anyhow::anyhow!("malformed account address: {err}"))?,
            mnemonic: value.mnemonic.parse().context("malformed mnemonic")?,
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
}

#[derive(FromRow)]
pub(crate) struct RawNyxd {
    #[allow(unused)]
    pub(crate) id: i64,
    pub(crate) rpc_endpoint: String,
    pub(crate) master_address: String,
}

#[derive(FromRow)]
#[allow(unused)]
pub(crate) struct LocalnetMetadata {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) created_at: OffsetDateTime,
}

#[derive(FromRow)]
pub(crate) struct StoredMetadata {
    pub(crate) latest_network_id: Option<i64>,
    pub(crate) latest_nyxd_id: Option<i64>,
}

#[derive(FromRow)]
pub(crate) struct RawNymApi {
    #[allow(unused)]
    pub(crate) network_id: i64,
    pub(crate) endpoint: String,
}

#[allow(unused)]
#[derive(FromRow)]
pub(crate) struct RawNymNode {
    #[allow(unused)]
    pub(crate) network_id: i64,
    pub(crate) node_id: i64,
    pub(crate) identity_key: String,
    pub(crate) private_identity_key: String,
    pub(crate) owner_address: String,
}
