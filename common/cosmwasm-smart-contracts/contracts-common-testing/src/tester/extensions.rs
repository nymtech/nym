// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ContractTester, TestableNymContract};
use cosmwasm_std::Addr;

pub trait AdminExt {
    fn admin(&self, storage_key: impl AsRef<[u8]>) -> Option<Addr>;

    fn admin_unchecked(&self, storage_key: impl AsRef<[u8]>) -> Addr {
        self.admin(storage_key).expect("no admin set")
    }
}

impl<C: TestableNymContract> AdminExt for ContractTester<C> {
    fn admin(&self, storage_key: impl AsRef<[u8]>) -> Option<Addr> {
        self.read_from_contract_storage(storage_key)
    }
}
