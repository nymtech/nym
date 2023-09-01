// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_contracts_common::IdentityKey;

use crate::location::LocationCache;

pub(crate) type GatewayLocationCache = LocationCache<IdentityKey>;
