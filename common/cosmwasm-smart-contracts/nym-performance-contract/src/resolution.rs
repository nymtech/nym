// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{EpochId, KindMedians, ResolvedMedians};
use cosmwasm_std::StdResult;

/// The medians rewarding uses for a node at `epoch_id`, reading each lookup only when needed.
///
/// `last_known` is only a starting hint: it names the node's newest bundle, so the walk meets
/// the same bundle first with or without it.
pub fn resolve_source(
    epoch_id: EpochId,
    medians_at: impl FnMut(EpochId) -> StdResult<Option<KindMedians>>,
    newest_measured: impl FnOnce(EpochId) -> StdResult<Option<EpochId>>,
    last_known: impl FnOnce() -> StdResult<Option<EpochId>>,
) -> StdResult<Option<ResolvedMedians>> {
    let _ = (epoch_id, medians_at, newest_measured, last_known);
    todo!("task 1.1")
}
