// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_bin_common::version_checker;
use time::OffsetDateTime;

pub(crate) fn refreshed_at(
    iter: impl IntoIterator<Item = OffsetDateTime>,
) -> OffsetDateTimeJsonSchemaWrapper {
    iter.into_iter().min().unwrap().into()
}

pub(crate) fn semver(requirement: &Option<String>, declared: &str) -> bool {
    if let Some(semver_compat) = requirement.as_ref() {
        if !version_checker::is_minor_version_compatible(declared, semver_compat) {
            return false;
        }
    }
    true
}
