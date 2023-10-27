// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum MixnodeError {
    // TODO: in the future this should work the other way, i.e. NymNode depending on Gateway errors
    #[error(transparent)]
    NymNodeError(#[from] nym_node::error::NymNodeError),
}
