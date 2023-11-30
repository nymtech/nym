// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::coconut::BlindSignRequestBody;
use nym_coconut::BlindedSignature;

pub struct InternalIssuedCredential {}

impl InternalIssuedCredential {
    pub fn new(request_body: BlindSignRequestBody, blinded_signature: &BlindedSignature) -> Self {
        todo!()
    }
}
