// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::State;
use crate::coconut::error::CoconutError;

pub(crate) async fn verification_key_submission(
    _dkg_client: &DkgClient,
    _state: &mut State,
) -> Result<(), CoconutError> {
    Ok(())
}
