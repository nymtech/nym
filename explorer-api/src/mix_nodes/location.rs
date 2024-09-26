// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::NodeId;

use crate::location::LocationCache;

pub(crate) type MixnodeLocationCache = LocationCache<NodeId>;
