// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use ipnetwork::IpNetwork;
use std::str::FromStr;

// used for parsing file content
#[derive(Debug)]
pub(crate) enum Host {
    Domain(String),
    IpNetwork(IpNetwork),
}

impl<S: AsRef<str>> From<S> for Host {
    fn from(raw: S) -> Self {
        // SAFETY: unwrap here is fine as `FromStr` implementation returns `Infallible` error.
        raw.as_ref().parse().unwrap()
    }
}

impl FromStr for Host {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(ipnet) = s.parse() {
            Ok(Host::IpNetwork(ipnet))
        } else {
            // TODO: perhaps in the future it should do some domain validation?
            //
            // So for example if somebody put some nonsense in the whitelist file like "foomp",
            // it would get rejected?
            Ok(Host::Domain(s.to_string()))
        }
    }
}
