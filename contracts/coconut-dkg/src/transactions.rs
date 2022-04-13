// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::BLACKLISTED_DEALERS;
use crate::ContractError;
use cosmwasm_std::{ensure, Addr, Deps, DepsMut, Env, MessageInfo, Response, Storage};

// currently we only require that
// a) it's a validator
// b) it wasn't blacklisted
fn verify_dealer(deps: Deps<'_>, dealer: &Addr) -> Result<(), ContractError> {
    if let Some(blacklisting) = BLACKLISTED_DEALERS.may_load(deps.storage, dealer)? {
        return Err(ContractError::BlacklistedDealer {
            reason: blacklisting,
        });
    }
    let all_validators = deps.querier.query_all_validators()?;
    if !all_validators
        .iter()
        .any(|validator| validator.address == dealer.as_ref())
    {
        return Err(ContractError::NotAValidator);
    }

    Ok(())
}

pub fn try_add_dealer(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    verify_dealer(deps.as_ref(), &info.sender)?;

    todo!()
}
