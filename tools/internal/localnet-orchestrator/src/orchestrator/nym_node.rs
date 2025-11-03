// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::orchestrator::account::Account;
use nym_coconut_dkg_common::types::Addr;
use nym_contracts_common::Percent;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::{NodeCostParams, NodeId, construct_nym_node_bonding_sign_payload};
use nym_validator_client::nyxd::CosmWasmCoin;
use std::net::IpAddr;

pub(crate) struct LocalnetNymNode {
    pub(crate) id: NodeId,

    pub(crate) gateway: bool,
    pub(crate) identity: ed25519::KeyPair,
    pub(crate) owner: Account,
}

impl LocalnetNymNode {
    pub(crate) fn pledge(&self) -> CosmWasmCoin {
        CosmWasmCoin::new(100_000000u32, "unym")
    }

    pub(crate) fn bonding_nym_node(&self, node_ip: IpAddr) -> nym_mixnet_contract_common::NymNode {
        nym_mixnet_contract_common::NymNode {
            host: node_ip.to_string(),
            custom_http_port: None,
            identity_key: self.identity.public_key().to_base58_string(),
        }
    }

    pub(crate) fn cost_params(&self) -> NodeCostParams {
        // SAFETY: we're using valid value
        #[allow(clippy::unwrap_used)]
        NodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
            interval_operating_cost: CosmWasmCoin::new(40_000000u32, "unym"),
        }
    }

    pub(crate) fn node_bonding_payload(&self, node_ip: IpAddr) -> String {
        let payload = construct_nym_node_bonding_sign_payload(
            0,
            Addr::unchecked(self.owner.address.to_string()),
            self.pledge(),
            self.bonding_nym_node(node_ip),
            self.cost_params(),
        );
        // SAFETY: we're using valid encoding
        #[allow(clippy::unwrap_used)]
        payload.to_base58_string().unwrap()
    }
}
