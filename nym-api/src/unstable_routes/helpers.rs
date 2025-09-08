// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use time::OffsetDateTime;

pub(crate) fn refreshed_at(
    iter: impl IntoIterator<Item = OffsetDateTime>,
) -> OffsetDateTimeJsonSchemaWrapper {
    iter.into_iter()
        .min()
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .into()
}
