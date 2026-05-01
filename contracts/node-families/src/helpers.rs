// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

/// Normalise a family name into the canonical form used as the unique-index key.
///
/// Drops every character that isn't an ASCII letter or digit and lowercases
/// the rest, so `"  Foo-Bar! "`, `"foobar"` and `"FOO BAR"` all collide on
/// the storage layer's unique-name index. Callers should pass the normalised
/// value to [`node_families_contract_common::NodeFamily::name`] when creating a family and when looking one
/// up by name.
pub fn normalise_family_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}
