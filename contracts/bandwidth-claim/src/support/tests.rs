// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
pub mod helpers {
    use crate::instantiate;
    use bandwidth_claim_contract::keys::PublicKey;
    use bandwidth_claim_contract::msg::InstantiateMsg;
    use bandwidth_claim_contract::payment::Payment;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
    use cosmwasm_std::{Empty, MemoryStorage, OwnedDeps};

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        return deps;
    }

    pub fn payment_fixture() -> Payment {
        let public_key = PublicKey::new([1; 32]);
        let gateway_identity = PublicKey::new([2; 32]);
        let bandwidth = 42;
        Payment::new(public_key, gateway_identity, bandwidth)
    }
}
