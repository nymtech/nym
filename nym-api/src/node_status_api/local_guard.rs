// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug)]
pub struct NonLocalRequestError;

/// Request guard that only allows requests coming from a local address
pub(crate) struct LocalRequest;

fn is_local_address(ip: Option<IpAddr>) -> bool {
    if let Some(address) = ip {
        match address {
            IpAddr::V4(ip) => ip == Ipv4Addr::LOCALHOST,
            IpAddr::V6(ip) => ip == Ipv6Addr::LOCALHOST,
        }
    } else {
        false
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for LocalRequest {
    type Error = NonLocalRequestError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if is_local_address(request.client_ip()) {
            Outcome::Success(LocalRequest)
        } else {
            warn!(
                "Received a request from {:?} for a local-only route",
                request.client_ip()
            );
            Outcome::Error((Status::Unauthorized, NonLocalRequestError))
        }
    }
}
