// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::manager::contract::Account;
use nym_coconut_dkg_common::types::Addr;
use nym_contracts_common::signing::MessageSignature;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::{construct_nym_node_bonding_sign_payload, NodeCostParams};
use nym_validator_client::nyxd::CosmWasmCoin;

pub(crate) struct NymNode {
    // host is always 127.0.0.1
    pub(crate) mix_port: u16,
    pub(crate) verloc_port: u16,
    pub(crate) http_port: u16,
    pub(crate) clients_port: u16,
    pub(crate) sphinx_key: String,
    pub(crate) identity_key: String,
    pub(crate) version: String,

    pub(crate) owner: Account,
    pub(crate) bonding_signature: String,
}

impl NymNode {
    pub(crate) fn new_empty() -> NymNode {
        NymNode {
            mix_port: 0,
            verloc_port: 0,
            http_port: 0,
            clients_port: 0,
            sphinx_key: "".to_string(),
            identity_key: "".to_string(),
            version: "".to_string(),
            owner: Account::new(),
            bonding_signature: "".to_string(),
        }
    }

    pub(crate) fn pledge(&self) -> CosmWasmCoin {
        CosmWasmCoin::new(100_000000, "unym")
    }

    pub(crate) fn bonding_nym_node(&self) -> nym_mixnet_contract_common::NymNode {
        nym_mixnet_contract_common::NymNode {
            host: "127.0.0.1".to_string(),
            custom_http_port: Some(self.http_port),
            identity_key: self.identity_key.clone(),
        }
    }

    pub(crate) fn cost_params(&self) -> NodeCostParams {
        NodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
            interval_operating_cost: CosmWasmCoin::new(40_000000, "unym"),
        }
    }

    pub(crate) fn bonding_signature(&self) -> MessageSignature {
        // this is a valid bs58 string
        self.bonding_signature.parse().unwrap()
    }

    pub(crate) fn bonding_payload(&self) -> String {
        let payload = construct_nym_node_bonding_sign_payload(
            0,
            Addr::unchecked(self.owner.address.to_string()),
            self.pledge(),
            self.bonding_nym_node(),
            self.cost_params(),
        );
        payload.to_base58_string().unwrap()
    }
}
