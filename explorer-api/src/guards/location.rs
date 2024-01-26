// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::geo_ip::location::{GeoIpError, Location};
use crate::state::ExplorerApiStateContext;
use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::request::Outcome;
use rocket::Request;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

const IP_HEADER: &str = "X-Real-IP";

#[derive(Debug)]
pub enum LocationError {
    NoIP,
    LocationNotFound,
    InternalError,
}

fn find_location(request: &Request<'_>) -> Result<Location, (Status, LocationError)> {
    let ip = request
        .headers()
        .get_one(IP_HEADER)
        .map(|f| f.to_string())
        .ok_or_else(|| {
            error!("Header not found, {}", IP_HEADER);
            (Status::Forbidden, LocationError::NoIP)
        })?;

    let geo_ip = &request
        .rocket()
        .state::<ExplorerApiStateContext>()
        .ok_or((Status::InternalServerError, LocationError::InternalError))? // should never fail
        .inner
        .geo_ip;

    let location = geo_ip
        .0
        .clone()
        .query(&ip, None)
        .map_err(|e| match e {
            GeoIpError::NoValidIP => (Status::Forbidden, LocationError::NoIP),
            GeoIpError::InternalError => {
                (Status::InternalServerError, LocationError::InternalError)
            }
        })?
        .ok_or_else(|| {
            warn!("Fail to find a matching location for {}", ip);
            (Status::Forbidden, LocationError::LocationNotFound)
        })?;
    Ok(location)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Location {
    type Error = LocationError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match find_location(request) {
            Ok(loc) => Outcome::Success(loc),
            Err(e) => Outcome::Error(e),
        }
    }
}

impl<'a> OpenApiFromRequest<'a> for Location {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}
