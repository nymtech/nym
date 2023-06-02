// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{constants::SIGNING_NONCES_NAMESPACE, Result};

use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::Map;
use nym_contracts_common::signing::Nonce;

pub const NONCES: Map<'_, Addr, Nonce> = Map::new(SIGNING_NONCES_NAMESPACE);

pub fn get_signing_nonce(storage: &dyn Storage, address: Addr) -> Result<Nonce> {
    let nonce = NONCES.may_load(storage, address)?.unwrap_or(0);
    Ok(nonce)
}

fn update_signing_nonce(storage: &mut dyn Storage, address: Addr, value: Nonce) -> Result<()> {
    NONCES
        .save(storage, address, &value)
        .map_err(|err| err.into())
}

pub fn increment_signing_nonce(storage: &mut dyn Storage, address: Addr) -> Result<()> {
    // get the current nonce
    let nonce = get_signing_nonce(storage, address.clone())?;

    // increment it for the next use
    update_signing_nonce(storage, address, nonce + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::transactions::instantiate_test_contract;
    use cosmwasm_std::{
        testing::{MockApi, MockQuerier},
        MemoryStorage, OwnedDeps,
    };
    use rstest::rstest;

    type TestDeps = OwnedDeps<MemoryStorage, MockApi, MockQuerier>;

    #[rstest::fixture]
    fn deps() -> TestDeps {
        instantiate_test_contract()
    }

    fn addr(s: &str) -> Addr {
        Addr::unchecked(s)
    }

    #[rstest]
    fn getting_signing_nonce_doesnt_increment_it(deps: TestDeps) {
        assert_eq!(get_signing_nonce(&deps.storage, addr("gunnar")).unwrap(), 0);
        assert_eq!(get_signing_nonce(&deps.storage, addr("gunnar")).unwrap(), 0);
    }

    #[rstest]
    fn increment_works(mut deps: TestDeps) {
        assert_eq!(get_signing_nonce(&deps.storage, addr("gunnar")).unwrap(), 0);
        increment_signing_nonce(&mut deps.storage, addr("gunnar")).unwrap();
        assert_eq!(get_signing_nonce(&deps.storage, addr("gunnar")).unwrap(), 1);
    }

    #[rstest]
    fn incrementing_is_independent(mut deps: TestDeps) {
        increment_signing_nonce(&mut deps.storage, addr("gunnar")).unwrap();
        assert_eq!(get_signing_nonce(&deps.storage, addr("gunnar")).unwrap(), 1);
        assert_eq!(get_signing_nonce(&deps.storage, addr("bjorn")).unwrap(), 0);
    }
}
