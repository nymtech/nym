// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Only non-parametrized (static) routes are defined here.
//! There may be other routes not present

// TODO dz consider moving to a more 'common' place?
macro_rules! absolute_route {
    ( $name:ident, $parent:expr, $suffix:expr ) => {
        pub fn $name() -> String {
            format!("{}{}", $parent, $suffix)
        }
    };
}

pub const V1: &str = "/v1";
// TODO dz do we really need this?
pub mod v1 {
    use super::*;

    pub const CIRCULATING_SUPPLY: &str = "/circulating-supply";

    absolute_route!(circulating_supply, V1, CIRCULATING_SUPPLY);

    pub mod circulating_supply {
        use super::*;

        pub const TOTAL_SUPPLY_VALUE: &str = "/total-supply-value";
        pub const CIRCULATING_SUPPLY_VALUE: &str = "/circulating-supply-value";

        absolute_route!(
            circulating_supply_value,
            circulating_supply(),
            CIRCULATING_SUPPLY_VALUE
        );
        absolute_route!(total_supply_value, circulating_supply(), TOTAL_SUPPLY_VALUE);
    }

    pub const MIXNODES: &str = "/mixnodes";
    pub const GATEWAYS: &str = "/gateways";
    pub const EPOCH: &str = "/epoch";
    pub const BLACKLISTED: &str = "/blacklisted";

    absolute_route!(epoch, V1, EPOCH);
    absolute_route!(mixnodes, V1, MIXNODES);
    absolute_route!(gateways, V1, GATEWAYS);

    pub mod mixnodes {
        use super::*;

        pub const DETAILED: &str = "/detailed";
        pub const ACTIVE: &str = "/active";
        pub const REWARDED: &str = "/rewarded";

        absolute_route!(detailed, mixnodes(), DETAILED);
        absolute_route!(active, mixnodes(), ACTIVE);
        absolute_route!(active_detailed, active(), DETAILED);
        absolute_route!(rewarded, mixnodes(), REWARDED);
        absolute_route!(rewarded_detailed, rewarded(), DETAILED);
        absolute_route!(blacklisted, mixnodes(), BLACKLISTED);
    }

    pub mod gateways {
        use super::*;

        pub const DESCRIBED: &str = "/described";

        absolute_route!(blacklisted, gateways(), BLACKLISTED);
        absolute_route!(described, gateways(), DESCRIBED);
    }

    pub mod epoch {
        use super::*;

        pub const REWARD_PARAMS: &str = "/reward_params";
        pub const CURRENT: &str = "/current";

        absolute_route!(reward_params, epoch(), REWARD_PARAMS);
        absolute_route!(current, epoch(), CURRENT);
    }

    pub const NETWORK: &str = "/network";
    absolute_route!(network, V1, NETWORK);

    pub mod network {
        use super::*;

        pub const DETAILS: &str = "/details";
        pub const NYM_CONTRACTS: &str = "/nym-contracts";
        pub const NYM_CONTRACTS_DETAILED: &str = "/nym-contracts-detailed";

        absolute_route!(details, network(), DETAILS);
        absolute_route!(nym_contracts, network(), NYM_CONTRACTS);
        absolute_route!(nym_contracts_detailed, network(), NYM_CONTRACTS_DETAILED);
    }

    pub const API_STATUS: &str = "/api-status";
    absolute_route!(api_status, V1, API_STATUS);

    pub mod api_status {
        use super::*;

        pub const HEALTH: &str = "/health";
        pub const BUILD_INFORMATION: &str = "/build-information";
        pub const SIGNER_INFORMATION: &str = "/signer-information";

        absolute_route!(health, api_status(), HEALTH);
        absolute_route!(build_information, api_status(), BUILD_INFORMATION);
        absolute_route!(signer_information, api_status(), SIGNER_INFORMATION);
    }
}
