// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::testing::{message_info, mock_dependencies, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    coins, Addr, BankMsg, CosmosMsg, Empty, Env, MemoryStorage, MessageInfo, OwnedDeps, Response,
};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub const TEST_DENOM: &str = "unym";

pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    rand_chacha::ChaCha20Rng::from_seed(dummy_seed)
}

pub fn deps_with_balance(env: &Env) -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::<Empty>::new(&[(
            env.contract.address.as_str(),
            coins(100000000000, TEST_DENOM).as_slice(),
        )]),
        custom_query_type: Default::default(),
    }
}

pub fn generate_sorted_addresses(n: usize) -> Vec<Addr> {
    let mut rng = test_rng();
    let mut addrs = Vec::with_capacity(n);
    for i in 0..n {
        addrs.push(MockApi::default().addr_make(&format!("addr{i}{}", rng.next_u64())));
    }
    addrs.sort();
    addrs
}

pub fn addr<S: AsRef<str>>(raw: S) -> Addr {
    mock_dependencies().api.addr_make(raw.as_ref())
}

pub fn sender<S: AsRef<str>>(raw: S) -> MessageInfo {
    message_info(&addr(raw), &[])
}

pub trait ExtractBankMsg {
    fn unwrap_bank_msg(self) -> Option<BankMsg>;
}

impl ExtractBankMsg for Response {
    fn unwrap_bank_msg(self) -> Option<BankMsg> {
        for msg in self.messages {
            match msg.msg {
                CosmosMsg::Bank(bank_msg) => return Some(bank_msg),
                _ => continue,
            }
        }

        None
    }
}
