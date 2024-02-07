// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub trait NymCredential {
    fn prove_credential(&self) -> Result<(), ()>;
}
