// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ip_info_lookup::client::IpInfoClient;
use crate::ip_info_lookup::models::LocationResponse;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use time::OffsetDateTime;

pub(crate) mod client;
pub(crate) mod models;

struct CachedResponse {
    at: OffsetDateTime,
    response: LocationResponse,
}

pub(crate) struct IpInfoLookup {
    client: IpInfoClient,
    cache_ttl: Duration,

    // cache of performed requests in case we had chain issues
    // note: this cache's TTL has to be way lower than the geodata TTL!
    // it should only exist for short-lived failures
    lookup_cache: HashMap<IpAddr, CachedResponse>,
}

impl IpInfoLookup {
    pub(crate) async fn lookup_address(&mut self, ip: IpAddr) -> anyhow::Result<LocationResponse> {
        if let Some(cached) = self.lookup_cache.get(&ip) {
            if cached.at + self.cache_ttl > OffsetDateTime::now_utc() {
                return Ok(cached.response.clone());
            }
        }

        let response = self.client.locate(ip).await?;
        self.lookup_cache.insert(
            ip,
            CachedResponse {
                at: OffsetDateTime::now_utc(),
                response: response.clone(),
            },
        );
        Ok(response)
    }
}
