// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{NymTopology, NymTopologyError};
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_routing::SphinxRouteMaker;
use nym_sphinx_types::Node;
use rand::{CryptoRng, Rng};

#[allow(dead_code)]
pub struct NymTopologyRouteProvider<R> {
    rng: R,
    inner: NymTopology,
}

impl<R> SphinxRouteMaker for NymTopologyRouteProvider<R>
where
    R: Rng + CryptoRng,
{
    type Error = NymTopologyError;

    fn sphinx_route(
        &mut self,
        hops: u8,
        destination: &Recipient,
    ) -> Result<Vec<Node>, NymTopologyError> {
        self.inner
            .random_route_to_gateway(&mut self.rng, hops, destination.gateway())
    }
}
