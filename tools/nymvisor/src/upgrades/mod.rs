// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;

mod serde_helpers;
pub(crate) mod types;

pub(crate) fn upgrade_binary() -> Result<(), NymvisorError> {
    // lock

    /*
    if binary already exist => swap symlink, write history and we're done
    otherwise deal with download


     */

    todo!()
}
