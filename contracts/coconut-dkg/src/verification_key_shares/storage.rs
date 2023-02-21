// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::verification_key::ContractVKShare;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

pub(crate) const VERIFICATION_KEY_SHARES_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const VERIFICATION_KEY_SHARES_PAGE_DEFAULT_LIMIT: u32 = 50;

type VKShareKey<'a> = &'a Addr;

pub(crate) const VK_SHARES: Map<'_, VKShareKey<'_>, ContractVKShare> = Map::new("vks");
