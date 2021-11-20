// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0










// storage prefixes
// all of them must be unique and presumably not be a prefix of a different one
// keeping them as short as possible is also desirable as they are part of each stored key
// it's not as important for singletons, but is a nice optimisation for buckets

// singletons
pub const CONFIG_KEY: &[u8] = b"config";
pub const LAYER_DISTRIBUTION_KEY: &[u8] = b"layers";
pub const REWARD_POOL_PREFIX: &[u8] = b"pool";

// buckets
pub const PREFIX_MIXNODES: &[u8] = b"mn";
pub const PREFIX_MIXNODES_OWNERS: &[u8] = b"mo";
pub const PREFIX_GATEWAYS: &[u8] = b"gt";
pub const PREFIX_GATEWAYS_OWNERS: &[u8] = b"go";

pub const PREFIX_MIX_DELEGATION: &[u8] = b"md";
pub const PREFIX_REVERSE_MIX_DELEGATION: &[u8] = b"dm";

pub const PREFIX_REWARDED_MIXNODES: &[u8] = b"rm";
