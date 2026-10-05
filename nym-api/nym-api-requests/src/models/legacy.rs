// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::{GatewayBond, MixNodeDetails, NodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyGatewayBondWithId {
    #[serde(flatten)]
    pub bond: GatewayBond,
    pub node_id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyMixnodesResponse {
    pub count: usize,
    pub nodes: Vec<MixNodeDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyGatewaysResponse {
    pub count: usize,
    pub nodes: Vec<LegacyGatewayBondWithId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coin, Addr};
    use nym_mixnet_contract_common::Gateway;

    // The bond is flattened: its fields sit alongside `node_id` at the top level rather than
    // nested under a `bond` key. This is the published shape of `/v1/gateways`, so it must not
    // move - notably, the on-disk cache stores its own non-flattened shape precisely so that
    // this one is free to stay flat.
    #[test]
    fn gateway_bond_with_id_serialises_flat() {
        let with_id = LegacyGatewayBondWithId {
            bond: GatewayBond::new(
                coin(100_000_000, "unym"),
                Addr::unchecked("n1owner"),
                1234,
                Gateway {
                    host: "1.1.1.1".to_string(),
                    mix_port: 1789,
                    clients_port: 9000,
                    location: "GB".to_string(),
                    sphinx_key: "sphinx".to_string(),
                    identity_key: "identity".to_string(),
                    version: "1.1.5".to_string(),
                },
            ),
            node_id: 42,
        };

        let json = serde_json::to_value(&with_id).unwrap();

        assert_eq!(json["node_id"], 42);
        // the bond's own fields are top-level, not under "bond"
        assert!(
            json.get("bond").is_none(),
            "the bond must stay flattened, got: {json}"
        );
        assert_eq!(json["block_height"], 1234);
        assert_eq!(json["owner"], "n1owner");
        assert_eq!(json["pledge_amount"]["denom"], "unym");
        assert_eq!(json["gateway"]["identity_key"], "identity");
    }
}
