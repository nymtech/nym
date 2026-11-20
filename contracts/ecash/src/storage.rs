// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::deposit::DepositStorage;
use crate::deposit_stats::DepositStatsStorage;
use crate::helpers::{BlacklistKey, Config};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Deps, DepsMut, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use nym_ecash_contract_common::blacklist::Blacklisting;
use nym_ecash_contract_common::counters::PoolCounters;
use nym_ecash_contract_common::EcashContractError;
use nym_network_defaults::TICKETBOOK_SIZE;

pub const ECASH_CONTRACT_STORAGE: EcashContractStorage = EcashContractStorage::new();

/// Snapshot of network-defaults values that the contract considers immutable
/// over its lifetime. Persisted at the `"expected_invariants"` storage key on
/// instantiation; every priced operation cross-checks the snapshot against
/// the current crate constant.
#[cw_serde]
pub(crate) struct Invariants {
    /// `nym_network_defaults::TICKETBOOK_SIZE` at instantiation time. Mismatch
    /// against the live constant trips `TicketBookSizeChanged`.
    pub(crate) ticket_book_size: u64,
}

pub struct EcashContractStorage {
    pub(crate) contract_admin: Admin,
    pub(crate) multisig: Admin,
    pub(crate) config: Item<Config>,
    pub(crate) pool_counters: Item<PoolCounters>,
    pub(crate) expected_invariants: Item<Invariants>,

    /// Information about the performed deposits
    pub(crate) deposit_stats: DepositStatsStorage,

    /// Map of approved addresses that are allowed to perform deposits using a reduced amount
    /// as specified by the saved value.
    pub(crate) reduced_deposits: Map<Addr, Coin>,

    pub(crate) blacklist: Map<BlacklistKey, Blacklisting>,

    pub(crate) deposits: DepositStorage,
}

impl EcashContractStorage {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            contract_admin: Admin::new("contract_admin"),
            multisig: Admin::new("multisig"),
            config: Item::new("config"),
            pool_counters: Item::new("pool_counters"),
            expected_invariants: Item::new("expected_invariants"),
            deposit_stats: DepositStatsStorage::new(),
            reduced_deposits: Map::new("reduced_deposits"),
            blacklist: Map::new("blacklist"),
            deposits: DepositStorage::new(),
        }
    }

    /// One-time storage initialisation called from the contract's `instantiate`
    /// entry point. Sets `sender` as the contract admin, persists the multisig
    /// pointer and the config, snapshots the ticketbook-size invariant, and
    /// zero-initialises the pool counters and default-tier stats.
    pub(crate) fn initialise(
        &self,
        mut deps: DepsMut,
        sender: Addr,
        multisig_addr: Addr,
        config: Config,
    ) -> Result<(), EcashContractError> {
        // all counters, deposits, etc. always use and always will use the same denom
        let zero_coin = coin(0, &config.deposit_amount.denom);

        // by default the sender becomes the admin
        self.contract_admin.set(deps.branch(), Some(sender))?;
        self.multisig.set(deps.branch(), Some(multisig_addr))?;

        self.expected_invariants.save(
            deps.storage,
            &Invariants {
                ticket_book_size: TICKETBOOK_SIZE,
            },
        )?;

        self.pool_counters.save(
            deps.storage,
            &PoolCounters {
                total_deposited: zero_coin.clone(),
                total_redeemed: zero_coin.clone(),
                tickets_requested_and_not_redeemed: 0,
            },
        )?;

        self.config.save(deps.storage, &config)?;

        self.deposit_stats
            .deposits_with_default_price
            .save(deps.storage, &0)?;

        self.deposit_stats
            .deposits_with_default_price_amounts
            .save(deps.storage, &zero_coin)?;

        Ok(())
    }

    /// Return `nym_network_defaults::TICKETBOOK_SIZE` if it matches the value
    /// snapshotted at instantiation; otherwise surface `TicketBookSizeChanged`
    /// so the caller halts before any state mutation.
    pub(crate) fn get_ticketbook_size(
        &self,
        storage: &dyn Storage,
    ) -> Result<u64, EcashContractError> {
        let invariants = self.expected_invariants.load(storage)?;
        if invariants.ticket_book_size != TICKETBOOK_SIZE {
            return Err(EcashContractError::TicketBookSizeChanged {
                at_init: invariants.ticket_book_size,
                current: TICKETBOOK_SIZE,
            });
        }
        Ok(TICKETBOOK_SIZE)
    }

    pub(crate) fn must_get_multisig_addr(&self, deps: Deps) -> Result<Addr, EcashContractError> {
        // SAFETY: multisig admin MUST always be set on initialisation,
        // if the call fails, we're in some weird UB land
        #[allow(clippy::expect_used)]
        Ok(self
            .multisig
            .get(deps)?
            .expect("multisig admin must always be set on initialisation"))
    }
}
