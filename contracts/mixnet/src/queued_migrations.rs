// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::DepsMut;
use mixnet_contract_common::error::MixnetContractError;

pub(crate) fn explicit_contract_admin(deps: DepsMut) -> Result<(), MixnetContractError> {
    // we need to read the deprecated field to migrate it over
    #[allow(deprecated)]
    // SAFETY: this value should ALWAYS exist on the first execution of this migration;
    // as a matter of fact, it should ALWAYS continue existing until another migration
    #[allow(clippy::expect_used)]
    let existing_admin = mixnet_params_storage::CONTRACT_STATE
        .load(deps.storage)?
        .owner
        .expect("the contract state is corrupt - there's no admin set");
    mixnet_params_storage::ADMIN.set(deps, Some(existing_admin))?;
    Ok(())
}
