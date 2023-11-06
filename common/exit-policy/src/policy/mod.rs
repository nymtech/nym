// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// adapted from: https://github.com/dgoulet-tor/arti/tree/781dc4bd64f515f0c13ae9907c473c2bad8fbf71
// and https://github.com/torproject/tor/blob/3cb6a690be60fcdab60130402ff88dcfc0657596/contrib/or-tools/exitlist
// + https://github.com/torproject/tor/blob/3cb6a690be60fcdab60130402ff88dcfc0657596/src/feature/dirparse/policy_parse.c

mod address_policy;
mod error;

pub use address_policy::{
    AddressPolicy, AddressPolicyAction, AddressPolicyRule, AddressPortPattern, IpPattern, PortRange,
};
pub use error::PolicyError;
