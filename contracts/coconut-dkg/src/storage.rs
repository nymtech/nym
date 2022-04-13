// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::Blacklisting;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

pub(crate) const BLACKLISTED_DEALERS: Map<'_, &'_ Addr, Blacklisting> = Map::new("bld");
