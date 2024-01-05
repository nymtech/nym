// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cw_controllers::Admin;
use cw_storage_plus::Item;
use nym_coconut_dkg_common::types::State;

// unique items
pub const DKG_ADMIN: Admin = Admin::new("dkg-admin");

pub const STATE: Item<State> = Item::new("state");

pub const MULTISIG: Admin = Admin::new("multisig");
