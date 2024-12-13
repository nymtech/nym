// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::NymEcashContract;
use crate::helpers::Config;
use cosmwasm_std::{Addr, Coin, Decimal, DepsMut};
use cw4::Cw4Contract;
use cw_storage_plus::Item;
use nym_ecash_contract_common::EcashContractError;
use serde::{Deserialize, Serialize};

pub fn remove_redemption_gateway_share(deps: DepsMut) -> Result<(), EcashContractError> {
    #[derive(Serialize, Deserialize)]
    struct OldConfig {
        group_addr: Cw4Contract,
        holding_account: Addr,

        redemption_gateway_share: Decimal,
        deposit_amount: Coin,
    }

    impl From<OldConfig> for Config {
        fn from(config: OldConfig) -> Self {
            Config {
                group_addr: config.group_addr,
                holding_account: config.holding_account,
                deposit_amount: config.deposit_amount,
            }
        }
    }

    const OLD_CONFIG: Item<OldConfig> = Item::new("config");

    let old_config = OLD_CONFIG.load(deps.storage)?;
    let new_config = old_config.into();

    NymEcashContract::new()
        .config
        .save(deps.storage, &new_config)?;

    Ok(())
}
