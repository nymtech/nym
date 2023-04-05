// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

pub struct DirectSigningNyxdClient {}

pub trait DkgQueryClient {}

// impl CosmWasmClient for DirectSigningNyxdClient {}

#[derive(Clone)]
pub struct Client<C> {
    _phantom: PhantomData<C>,
}

impl<C> DkgQueryClient for Client<C> {}
