// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::manager::contract::Account;
use nym_coconut_dkg_common::types::Addr;
use nym_contracts_common::signing::MessageSignature;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::{
    construct_gateway_bonding_sign_payload, construct_mixnode_bonding_sign_payload, Gateway,
    MixNode, MixNodeCostParams,
};
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

    pub(crate) fn gateway(&self) -> Gateway {
        Gateway {
            host: "127.0.0.1".to_string(),
            mix_port: self.mix_port,
            clients_port: self.clients_port,
            location: "foomp".to_string(),
            sphinx_key: self.sphinx_key.clone(),
            identity_key: self.identity_key.clone(),
            version: self.version.clone(),
        }
    }

    pub(crate) fn mixnode(&self) -> MixNode {
        MixNode {
            host: "127.0.0.1".to_string(),
            mix_port: self.mix_port,
            verloc_port: self.verloc_port,
            http_api_port: self.http_port,
            sphinx_key: self.sphinx_key.clone(),
            identity_key: self.identity_key.clone(),
            version: self.version.clone(),
        }
    }

    pub(crate) fn cost_params(&self) -> MixNodeCostParams {
        MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
            interval_operating_cost: CosmWasmCoin::new(40_000000, "unym"),
        }
    }

    pub(crate) fn bonding_signature(&self) -> MessageSignature {
        // this is a valid bs58 string
        self.bonding_signature.parse().unwrap()
    }

    pub(crate) fn mixnode_bonding_payload(&self) -> String {
        let payload = construct_mixnode_bonding_sign_payload(
            0,
            Addr::unchecked(self.owner.address.to_string()),
            None,
            self.pledge(),
            self.mixnode(),
            self.cost_params(),
        );
        payload.to_base58_string().unwrap()
    }

    pub(crate) fn gateway_bonding_payload(&self) -> String {
        let payload = construct_gateway_bonding_sign_payload(
            0,
            Addr::unchecked(self.owner.address.to_string()),
            None,
            self.pledge(),
            self.gateway(),
        );
        payload.to_base58_string().unwrap()
    }
}
