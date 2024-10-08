// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct ExecuteMsg {
    pub pairs: Vec<(String, String)>,
}

impl From<Vec<(Vec<u8>, Vec<u8>)>> for ExecuteMsg {
    fn from(raw: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
        ExecuteMsg {
            pairs: raw
                .into_iter()
                .map(|(k, v)| (base85::encode(&k), base85::encode(&v)))
                .collect(),
        }
    }
}
