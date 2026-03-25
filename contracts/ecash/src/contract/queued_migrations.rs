// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::NymEcashContract;
use cosmwasm_std::DepsMut;
use nym_ecash_contract_common::msg::WhitelistedDeposit;
use nym_ecash_contract_common::EcashContractError;

pub fn add_tiered_pricing(
    mut deps: DepsMut,
    initial_whitelist: Vec<WhitelistedDeposit>,
) -> Result<(), EcashContractError> {
    let contract = NymEcashContract::new();

    // All the deposits made so far were performed with the default price.
    let deposits_performed = contract.deposits.total_deposits_made(deps.storage)?;
    let deposits_amounts = contract.pool_counters.load(deps.storage)?.total_deposited;

    contract
        .deposit_stats
        .deposits_with_default_price
        .save(deps.storage, &deposits_performed)?;

    contract
        .deposit_stats
        .deposits_with_default_price_amounts
        .save(deps.storage, &deposits_amounts)?;

    // Seed the whitelist with the initial set of reduced deposit prices.
    for whitelisted in initial_whitelist {
        let addr = deps.api.addr_validate(&whitelisted.address)?;

        contract.add_reduced_deposit_address(deps.branch(), addr, &whitelisted.deposit)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::helpers::Invariants;
    use crate::deposit::DepositStorage;
    use crate::deposit_stats::DepositStatsStorage;
    use crate::helpers::Config;
    use cosmwasm_std::testing::{mock_dependencies, MockApi, MockQuerier};
    use cosmwasm_std::{coin, Empty, MemoryStorage, OwnedDeps, Uint128};
    use cw4::Cw4Contract;
    use cw_storage_plus::Item;
    use nym_ecash_contract_common::counters::PoolCounters;

    const DENOM: &str = "unym";
    const DEFAULT_DEPOSIT: u128 = 75_000_000;
    const TICKET_BOOK_SIZE: u64 = 50;

    /// Initialise the contract config and invariants so that whitelist
    /// validation during migration has the values it needs.
    fn save_config_and_invariants(
        deps: &mut OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>>,
    ) {
        let contract = NymEcashContract::new();
        let group_addr = deps.api.addr_make("group");
        let holding_account = deps.api.addr_make("holding");
        contract
            .config
            .save(
                deps.as_mut().storage,
                &Config {
                    group_addr: Cw4Contract(group_addr),
                    holding_account,
                    deposit_amount: coin(DEFAULT_DEPOSIT, DENOM),
                },
            )
            .unwrap();
        contract
            .expected_invariants
            .save(
                deps.as_mut().storage,
                &Invariants {
                    ticket_book_size: TICKET_BOOK_SIZE,
                },
            )
            .unwrap();
    }

    fn save_pool_counters(storage: &mut dyn cosmwasm_std::Storage, total_deposited: u128) {
        let pool_counters: Item<PoolCounters> = Item::new("pool_counters");
        pool_counters
            .save(
                storage,
                &PoolCounters {
                    total_deposited: coin(total_deposited, DENOM),
                    total_redeemed: coin(0, DENOM),
                },
            )
            .unwrap();
    }

    #[test]
    fn migration_with_no_prior_deposits_initialises_stats_to_zero() {
        let mut deps = mock_dependencies();

        // No deposit_id_counter saved — contract never had a deposit.
        save_pool_counters(deps.as_mut().storage, 0);

        add_tiered_pricing(deps.as_mut(), vec![]).unwrap();

        let stats = DepositStatsStorage::new();
        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            0
        );
        assert_eq!(
            stats
                .deposits_with_default_price_amounts
                .load(deps.as_ref().storage)
                .unwrap(),
            coin(0, DENOM)
        );
    }

    #[test]
    fn migration_with_prior_deposits_backfills_correct_count() {
        let mut deps = mock_dependencies();
        let n_deposits: u32 = 3;
        let total: u128 = n_deposits as u128 * 75_000_000;

        // Simulate n_deposits having been made: counter stores the next available id,
        // which equals the number of deposits already performed.
        let deposits = DepositStorage::new();
        deposits
            .deposit_id_counter
            .save(deps.as_mut().storage, &n_deposits)
            .unwrap();

        save_pool_counters(deps.as_mut().storage, total);

        add_tiered_pricing(deps.as_mut(), vec![]).unwrap();

        let stats = DepositStatsStorage::new();
        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            n_deposits
        );
        assert_eq!(
            stats
                .deposits_with_default_price_amounts
                .load(deps.as_ref().storage)
                .unwrap(),
            coin(total, DENOM)
        );
    }

    #[test]
    fn migration_with_single_deposit_backfills_count_of_one() {
        let mut deps = mock_dependencies();

        // After one deposit, next_id returns 0 and saves counter=1.
        let deposits = DepositStorage::new();
        deposits
            .deposit_id_counter
            .save(deps.as_mut().storage, &1u32)
            .unwrap();

        save_pool_counters(deps.as_mut().storage, 75_000_000);

        add_tiered_pricing(deps.as_mut(), vec![]).unwrap();

        let stats = DepositStatsStorage::new();
        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            1
        );
    }

    #[test]
    fn migration_stores_valid_whitelist_entries() {
        let mut deps = mock_dependencies();
        save_pool_counters(deps.as_mut().storage, 0);
        save_config_and_invariants(&mut deps);

        let addr1 = deps.api.addr_make("alice");
        let addr2 = deps.api.addr_make("bob");

        let whitelist = vec![
            WhitelistedDeposit {
                address: addr1.to_string(),
                deposit: coin(10_000_000, DENOM),
            },
            WhitelistedDeposit {
                address: addr2.to_string(),
                deposit: coin(50_000_000, DENOM),
            },
        ];

        add_tiered_pricing(deps.as_mut(), whitelist).unwrap();

        let contract = NymEcashContract::new();
        assert_eq!(
            contract
                .reduced_deposits
                .load(deps.as_ref().storage, addr1)
                .unwrap(),
            coin(10_000_000, DENOM)
        );
        assert_eq!(
            contract
                .reduced_deposits
                .load(deps.as_ref().storage, addr2)
                .unwrap(),
            coin(50_000_000, DENOM)
        );
    }

    #[test]
    fn migration_rejects_wrong_denom() {
        let mut deps = mock_dependencies();
        save_pool_counters(deps.as_mut().storage, 0);
        save_config_and_invariants(&mut deps);

        let whitelist = vec![WhitelistedDeposit {
            address: deps.api.addr_make("alice").to_string(),
            deposit: coin(10_000_000, "uatom"),
        }];

        let err = add_tiered_pricing(deps.as_mut(), whitelist).unwrap_err();
        assert_eq!(
            err,
            EcashContractError::InvalidReducedDepositDenom {
                expected: DENOM.to_string(),
                got: "uatom".to_string(),
            }
        );
    }

    #[test]
    fn migration_rejects_amount_not_less_than_default() {
        let mut deps = mock_dependencies();
        save_pool_counters(deps.as_mut().storage, 0);
        save_config_and_invariants(&mut deps);

        // Equal to default — should fail
        let whitelist = vec![WhitelistedDeposit {
            address: deps.api.addr_make("alice").to_string(),
            deposit: coin(DEFAULT_DEPOSIT, DENOM),
        }];

        let err = add_tiered_pricing(deps.as_mut(), whitelist).unwrap_err();
        assert_eq!(
            err,
            EcashContractError::ReducedDepositNotReduced {
                reduced: Uint128::new(DEFAULT_DEPOSIT),
                default: Uint128::new(DEFAULT_DEPOSIT),
            }
        );

        // Greater than default — should also fail
        let whitelist = vec![WhitelistedDeposit {
            address: deps.api.addr_make("alice").to_string(),
            deposit: coin(DEFAULT_DEPOSIT + 1, DENOM),
        }];

        let err = add_tiered_pricing(deps.as_mut(), whitelist).unwrap_err();
        assert_eq!(
            err,
            EcashContractError::ReducedDepositNotReduced {
                reduced: Uint128::new(DEFAULT_DEPOSIT + 1),
                default: Uint128::new(DEFAULT_DEPOSIT),
            }
        );
    }

    #[test]
    fn migration_rejects_amount_below_ticket_book_size() {
        let mut deps = mock_dependencies();
        save_pool_counters(deps.as_mut().storage, 0);
        save_config_and_invariants(&mut deps);

        let whitelist = vec![WhitelistedDeposit {
            address: deps.api.addr_make("alice").to_string(),
            deposit: coin(TICKET_BOOK_SIZE as u128 - 1, DENOM),
        }];

        let err = add_tiered_pricing(deps.as_mut(), whitelist).unwrap_err();
        assert_eq!(
            err,
            EcashContractError::DepositBelowTicketBookSize {
                amount: Uint128::new(TICKET_BOOK_SIZE as u128 - 1),
                ticket_book_size: TICKET_BOOK_SIZE,
            }
        );
    }
}
