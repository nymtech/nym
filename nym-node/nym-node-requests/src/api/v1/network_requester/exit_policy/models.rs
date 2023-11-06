// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_exit_policy::{
    AddressPolicy, AddressPolicyAction, AddressPolicyRule, AddressPortPattern, ExitPolicy,
    PortRange,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UsedExitPolicy {
    /// Flag indicating whether this node uses the below exit policy or
    /// whether it still relies on the legacy allow lists.
    pub enabled: bool,

    /// Source URL from which the exit policy was obtained
    #[cfg_attr(
        feature = "openapi",
        schema(example = "https://nymtech.net/.wellknown/network-requester/exit-policy.txt")
    )]
    pub upstream_source: String,

    /// Unix timestamp indicating when the exit policy was last updated from the upstream.
    #[cfg_attr(feature = "openapi", schema(example = 1697731611))]
    pub last_updated: u64,

    /// The actual policy used by this node.
    // `ExitPolicy` is a type alias for `AddressPolicy`,
    // but it seems utoipa is too stupid to realise it by itself
    #[cfg_attr(feature = "openapi", schema(value_type = Option<AddressPolicy>))]
    pub policy: Option<ExitPolicy>,
}

impl Default for UsedExitPolicy {
    fn default() -> Self {
        UsedExitPolicy {
            enabled: false,
            upstream_source: "".to_string(),
            last_updated: 0,
            policy: None,
        }
    }
}
