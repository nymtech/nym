// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod coconut;
pub mod models;
pub mod pagination;

pub trait Deprecatable {
    fn deprecate(self) -> Deprecated<Self>
    where
        Self: Sized,
    {
        self.into()
    }
}

impl<T> Deprecatable for T {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Deprecated<T> {
    pub deprecated: bool,
    #[serde(flatten)]
    pub response: T,
}

impl<T> From<T> for Deprecated<T> {
    fn from(response: T) -> Self {
        Deprecated {
            deprecated: true,
            response,
        }
    }
}

macro_rules! absolute_route {
    ( $name:ident, $parent:expr, $suffix:expr ) => {
        pub fn $name() -> String {
            format!("{}{}", $parent, $suffix)
        }
    };
}

pub mod routes {
    pub const V1: &str = "/v1";

    pub mod v1 {
        use super::*;

        pub const CIRCULATING_SUPPLY: &str = "/circulating-supply";
        pub const MIXNODES: &str = "/mixnodes";
        pub const GATEWAYS: &str = "/gateways";
        pub const STATUS: &str = "/status";
        pub const EPOCH: &str = "/epoch";
        pub const NETWORK: &str = "/network";
        pub const API_STATUS: &str = "/api-status";

        pub mod circulating_supply {
            use super::*;

            pub const TOTAL_SUPPLY_VALUE: &str = "/total-supply-value";
            pub const CIRCULATING_SUPPLY_VALUE: &str = "/circulating-supply-value";
        }

        pub mod mixnodes {
            use super::*;

            pub const DETAILED: &str = "/detailed";
            pub const ACTIVE: &str = "/active";
            pub const REWARDED: &str = "/rewarded";
            pub const BLACKLISTED: &str = "/blacklisted";

            pub mod active {
                pub const DETAILED: &str = "/detailed";
            }

            pub mod rewarded {
                pub const DETAILED: &str = "/detailed";
            }
        }

        pub mod gateways {
            use super::*;

            pub const DETAILED: &str = "/detailed";
            pub const BLACKLISTED: &str = "/blacklisted";
            pub const DESCRIBED: &str = "/described";
        }

        pub mod epoch {
            use super::*;

            pub const REWARD_PARAMS: &str = "reward_params";
            pub const CURRENT: &str = "current";
        }

        pub mod status {
            use super::*;

            pub const GATEWAY: &str = "/gateway";
            pub const MIXNODE: &str = "/mixnode";
            pub const MIXNODES: &str = "/mixnode";
            pub const GATEWAYS: &str = "/gateways";
            pub const UNSTABLE: &str = "/unstable";

            pub mod gateway {
                pub const REPORT: &str = "/report";
                pub const HISTORY: &str = "/history";
                pub const CORE_STATUS_COUNT: &str = "/core-status-count";
                pub const AVG_UPTIME: &str = "/avg_uptime";
            }

            pub mod mixnode {
                pub const REPORT: &str = "/report";
                pub const HISTORY: &str = "/history";
                pub const CORE_STATUS_COUNT: &str = "/core-status-count";
                pub const STATUS: &str = "/status";
                pub const REWARD_ESTIMATION: &str = "/reward-estimation";
                pub const COMPUTE_REWARD_ESTIMATION: &str = "/compute-reward-estimation";
                pub const STAKE_SATURATION: &str = "/stake-saturation";

                pub const INCLUSION_PROBABILITY: &str = "/inclusion-probability";
                pub const AVG_UPTIME: &str = "/avg_uptime";
            }

            pub mod mixnodes {
                pub const INCLUSION_PROBABILITY: &str = "/inclusion-probability";

                pub const DETAILED: &str = "/detailed";
                pub const DETAILED_UNFILTERED: &str = "/detailed-unfiltered";

                pub const ACTIVE: &str = "/active";
                pub const REWARDED: &str = "/rewarded";
                pub mod rewarded {
                    pub const DETAILED: &str = "/detailed";
                }

                pub mod active {
                    pub const DETAILED: &str = "/detailed";
                }
            }

            pub mod gateways {
                pub const DETAILED: &str = "/detailed";
                pub const DETAILED_UNFILTERED: &str = "/detailed-unfiltered";
            }

            pub mod unstable {
                pub mod by_mix_id {
                    pub const TEST_RESULTS: &str = "/test-results";
                }
                pub mod by_gateway_identity {
                    pub const TEST_RESULTS: &str = "/test-results";
                }
            }
        }

        pub mod network {
            use super::*;
            pub const DETAILS: &str = "/details";
            pub const NYM_CONTRACTS: &str = "/nym-contracts";
            pub const NYM_CONTRACTS_DETAILED: &str = "/nym-contracts-detailed";
        }

        pub mod api_status {
            use super::*;
            pub const HEALTH: &str = "/health";
            pub const BUILD_INFORMATION: &str = "/build-information";
            pub const SIGNER_INFORMATION: &str = "/signer-information";
        }
    }
}
