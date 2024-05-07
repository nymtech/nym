// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::Result;
use crate::ecash::storage::models::IssuedCredential;
use nym_api_requests::coconut::models::IssuedCredentialBody;
use nym_api_requests::coconut::models::IssuedCredentialsResponse;
use std::collections::BTreeMap;

pub(crate) fn build_credentials_response(
    raw: Vec<IssuedCredential>,
) -> Result<IssuedCredentialsResponse> {
    let mut credentials = BTreeMap::new();

    for raw_credential in raw {
        let id = raw_credential.id;
        let api_issued = IssuedCredentialBody::try_from(raw_credential)?;
        let old = credentials.insert(id, api_issued);
        if old.is_some() {
            // why do we panic here rather than return an error? because it's a critical failure because
            // since the raw values came directly from the database with the PRIMARY KEY constraint
            // it should be IMPOSSIBLE to have duplicate values here
            panic!("somehow retrieved multiple credentials with id {id} from the database!")
        }
    }

    Ok(IssuedCredentialsResponse { credentials })
}
