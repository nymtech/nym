// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::NodeFamiliesStorage;
use cosmwasm_std::{Addr, Deps};
use nym_mixnet_contract_common::{MixnetContractQuerier, NodeId};
use nym_node_families_contract_common::{Config, NodeFamiliesContractError, NodeFamilyId};

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

/// A new `(display, normalised)` family name pair that has passed
/// length and non-empty-after-normalisation validation. Constructed by
/// [`validate_family_name`]; consumed by storage write paths that update
/// the family's `name` + `normalised_name` columns together.
pub(crate) struct NewFamilyName {
    pub(crate) name: String,
    pub(crate) normalised_name: String,
}

/// Validate a candidate family name against [`Config::family_name_length_limit`]
/// and the non-empty-after-normalisation rule, returning the paired display +
/// normalised forms. The caller is responsible for the uniqueness check
/// against existing families (see [`ensure_normalised_name_unique`]).
pub(crate) fn validate_family_name(
    name: String,
    config: &Config,
) -> Result<NewFamilyName, NodeFamiliesContractError> {
    if name.len() > config.family_name_length_limit {
        return Err(NodeFamiliesContractError::FamilyNameTooLong {
            length: name.len(),
            limit: config.family_name_length_limit,
        });
    }
    let normalised_name = normalise_family_name(&name);
    if normalised_name.is_empty() {
        return Err(NodeFamiliesContractError::EmptyFamilyName);
    }
    Ok(NewFamilyName {
        name,
        normalised_name,
    })
}

/// Validate a candidate family description against
/// [`Config::family_description_length_limit`].
pub(crate) fn validate_family_description(
    description: &str,
    config: &Config,
) -> Result<(), NodeFamiliesContractError> {
    if description.len() > config.family_description_length_limit {
        return Err(NodeFamiliesContractError::FamilyDescriptionTooLong {
            length: description.len(),
            limit: config.family_description_length_limit,
        });
    }
    Ok(())
}

/// Ensure no family other than `excluded_id` has `normalised_name` as its
/// normalised name. `excluded_id = None` is used on the create path (any
/// match is a collision); `excluded_id = Some(family.id)` on the update
/// path lets a family keep its current normalised key.
pub(crate) fn ensure_normalised_name_unique(
    storage: &NodeFamiliesStorage,
    deps: Deps,
    normalised_name: &str,
    excluded_id: Option<NodeFamilyId>,
) -> Result<(), NodeFamiliesContractError> {
    if let Some((_, existing)) = storage
        .families
        .idx
        .normalised_name
        .item(deps.storage, normalised_name.to_owned())?
    {
        if Some(existing.id) != excluded_id {
            return Err(NodeFamiliesContractError::FamilyNameAlreadyTaken {
                name: normalised_name.to_owned(),
                family_id: existing.id,
            });
        }
    }
    Ok(())
}

/// Resolve the bonded node (if any) that should be enrolled as the founding
/// member of a family `address` is about to create.
///
/// Returns `Ok(Some(node_id))` when `address` controls a bonded, not-unbonding
/// node that is not yet a member of any family. The caller enrols it as the
/// family's first member. Returns `Ok(None)` when `address` controls no bonded
/// node, or controls one that has already entered the unbonding state (a node
/// on its way out of the network is not auto-enrolled). Errors with
/// [`AlreadyInFamily`] if the controlled node already belongs to a family,
/// since an address cannot create a family around a node committed elsewhere.
///
/// [`AlreadyInFamily`]: NodeFamiliesContractError::AlreadyInFamily
pub(crate) fn resolve_founding_node(
    storage: &NodeFamiliesStorage,
    deps: Deps,
    address: &Addr,
) -> Result<Option<NodeId>, NodeFamiliesContractError> {
    let mixnet_contract = storage.mixnet_contract_address.load(deps.storage)?;
    let Some(nym_node) = deps
        .querier
        .query_nymnode_ownership(&mixnet_contract, address)?
    else {
        // if the owner has no nym-node, it can't possibly be in a family
        return Ok(None);
    };

    // check if that node is in a family
    if let Some(family) = storage
        .family_members
        .may_load(deps.storage, nym_node.node_id)?
    {
        return Err(NodeFamiliesContractError::AlreadyInFamily {
            address: address.clone(),
            node_id: nym_node.node_id,
            family_id: family.family_id,
        });
    }

    // a node leaving the network is not auto-added
    if nym_node.is_unbonding {
        return Ok(None);
    }

    Ok(Some(nym_node.node_id))
}

/// Cross-contract query: ensure `node_id` is a currently-bonded node in the
/// mixnet contract. Returns [`NodeDoesntExist`] otherwise.
///
/// [`NodeDoesntExist`]: NodeFamiliesContractError::NodeDoesntExist
pub(crate) fn ensure_node_is_bonded(
    storage: &NodeFamiliesStorage,
    deps: Deps,
    node_id: NodeId,
) -> Result<(), NodeFamiliesContractError> {
    let mixnet_contract = storage.mixnet_contract_address.load(deps.storage)?;
    if !deps
        .querier
        .check_node_existence(&mixnet_contract, node_id)?
    {
        return Err(NodeFamiliesContractError::NodeDoesntExist { node_id });
    }
    Ok(())
}

/// Ensure `address` is the controller of the bonded node `node_id` per the
/// mixnet contract. Errors with [`SenderDoesntControlNode`] when `address`
/// owns no bonded node, owns a node with a different id, or owns it but it
/// has entered the unbonding state.
///
/// [`SenderDoesntControlNode`]: NodeFamiliesContractError::SenderDoesntControlNode
pub(crate) fn ensure_has_bonded_node(
    storage: &NodeFamiliesStorage,
    deps: Deps,
    address: &Addr,
    node_id: NodeId,
) -> Result<(), NodeFamiliesContractError> {
    let mixnet_contract = storage.mixnet_contract_address.load(deps.storage)?;
    match deps
        .querier
        .query_nymnode_ownership(&mixnet_contract, address)?
    {
        Some(bond) if bond.node_id == node_id && !bond.is_unbonding => Ok(()),
        _ => Err(NodeFamiliesContractError::SenderDoesntControlNode {
            address: address.clone(),
            node_id,
        }),
    }
}

/// Ensure `node_id` is not currently a member of any family. Returns
/// [`NodeAlreadyInFamily`] if it is.
///
/// [`NodeAlreadyInFamily`]: NodeFamiliesContractError::NodeAlreadyInFamily
pub(crate) fn ensure_node_not_in_family(
    storage: &NodeFamiliesStorage,
    deps: Deps,
    node_id: NodeId,
) -> Result<(), NodeFamiliesContractError> {
    if let Some(membership) = storage.family_members.may_load(deps.storage, node_id)? {
        return Err(NodeFamiliesContractError::NodeAlreadyInFamily {
            node_id,
            family_id: membership.family_id,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod normalise_family_name {
        use super::*;

        #[test]
        fn empty_input_yields_empty() {
            assert_eq!(normalise_family_name(""), "");
        }

        #[test]
        fn already_canonical_is_unchanged() {
            assert_eq!(normalise_family_name("foobar42"), "foobar42");
        }

        #[test]
        fn lowercases_uppercase_letters() {
            assert_eq!(normalise_family_name("FOOBAR"), "foobar");
            assert_eq!(normalise_family_name("FooBar"), "foobar");
        }

        #[test]
        fn strips_whitespace() {
            assert_eq!(normalise_family_name("  foo bar  "), "foobar");
            assert_eq!(normalise_family_name("foo\tbar\nbaz"), "foobarbaz");
        }

        #[test]
        fn strips_punctuation_and_symbols() {
            assert_eq!(normalise_family_name("foo-bar!"), "foobar");
            assert_eq!(normalise_family_name("a.b_c@d"), "abcd");
        }

        #[test]
        fn preserves_digits() {
            assert_eq!(normalise_family_name("squad-2026"), "squad2026");
            assert_eq!(normalise_family_name("0123456789"), "0123456789");
        }

        #[test]
        fn drops_non_ascii_letters() {
            // is_ascii_alphanumeric is strict — accented and non-Latin chars are dropped.
            assert_eq!(normalise_family_name("café"), "caf");
            assert_eq!(normalise_family_name("Ω-team"), "team");
            assert_eq!(normalise_family_name("名前"), "");
        }

        #[test]
        fn all_symbols_input_normalises_to_empty() {
            // try_create_family relies on this to surface EmptyFamilyName.
            assert_eq!(normalise_family_name("   "), "");
            assert_eq!(normalise_family_name("!!!---"), "");
        }

        #[test]
        fn distinct_inputs_collide_under_normalisation() {
            // The collision behaviour the unique-name index depends on.
            let canonical = normalise_family_name("Foo Bar");
            assert_eq!(canonical, normalise_family_name("foobar"));
            assert_eq!(canonical, normalise_family_name("FOO-BAR"));
            assert_eq!(canonical, normalise_family_name("  f.o.o.b.a.r  "));
        }
    }
}
