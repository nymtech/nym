// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Binary, BlockInfo, Env, StdResult};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(feature = "rand")]
pub fn test_rng() -> rand_chacha::ChaCha20Rng {
    use rand_chacha::rand_core::SeedableRng;

    let dummy_seed = [42u8; 32];
    rand_chacha::ChaCha20Rng::from_seed(dummy_seed)
}

pub fn env_with_block_info(info: BlockInfo) -> Env {
    let mut env = mock_env();
    env.block = info;
    env
}

pub fn deserialize_msg<M: DeserializeOwned>(raw: &Binary) -> StdResult<M> {
    cosmwasm_std::from_binary(raw)
}

pub fn serialize_msg<M: Serialize>(msg: &M) -> StdResult<Binary> {
    cosmwasm_std::to_binary(msg)
}

// used only for purposes of providing more informative error messages
pub(crate) fn raw_msg_to_string(raw: &Binary) -> String {
    #[cfg(not(feature = "serde_json"))]
    return "<serde_json feature is not enabled - can't format the message>".to_string();

    #[cfg(feature = "serde_json")]
    match serde_json::from_slice::<serde_json::Value>(raw.as_slice()) {
        Ok(deserialized) => deserialized.to_string(),
        Err(_) => "ERR: COULD NOT RECOVER THE ORIGINAL MESSAGE".to_string(),
    }
}
