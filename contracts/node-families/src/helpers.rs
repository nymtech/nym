// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::{Addr, Deps};
use node_families_contract_common::NodeFamiliesContractError;

/// Normalise a family name into the canonical form used as the unique-index key.
///
/// Drops every character that isn't an ASCII letter or digit and lowercases
/// the rest, so `"  Foo-Bar! "`, `"foobar"` and `"FOO BAR"` all collide on
/// the storage layer's unique-name index.
pub fn normalise_family_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Ensure no node controlled by `address` is currently a member of any family.
///
/// Stub: the real implementation will cross-contract query the mixnet contract
/// for nodes owned by `address` and check each against `family_members`.
/// Returns `AlreadyInFamily` if any controlled node is already in
/// a family.
pub(crate) fn ensure_address_holds_no_family_membership(
    deps: Deps,
    address: &Addr,
) -> Result<(), NodeFamiliesContractError> {
    let _ = (deps, address);
    Ok(())
}
