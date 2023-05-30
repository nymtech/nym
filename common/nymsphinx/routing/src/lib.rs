// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_types::Node;
use thiserror::Error;

pub trait SphinxRouteMaker {
    type Error;

    fn sphinx_route(&mut self, hops: u8, destination: &Recipient)
        -> Result<Vec<Node>, Self::Error>;
}

#[derive(Debug, Error, Clone, Copy)]
#[error("the route vector contains {available} nodes while {requested} hops are required")]
pub struct InvalidNumberOfHops {
    available: usize,
    requested: u8,
}

// if one wants to provide a hardcoded route, they can
impl SphinxRouteMaker for Vec<Node> {
    type Error = InvalidNumberOfHops;

    fn sphinx_route(
        &mut self,
        hops: u8,
        _destination: &Recipient,
    ) -> Result<Vec<Node>, InvalidNumberOfHops> {
        // it's the responsibility of the caller to ensure the hardcoded route has correct number of hops
        // and that it's final hop include the recipient's gateway.

        if self.len() != hops as usize {
            Err(InvalidNumberOfHops {
                available: self.len(),
                requested: hops,
            })
        } else {
            Ok(self.clone())
        }
    }
}
