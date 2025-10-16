// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use nym_common::trace_err_chain;
use nym_http_api_client::HickoryDnsResolver;

use crate::{Config, Error, error::Result, gateway_client::ResolvedConfig};

// be generous with the resolution timeout
const HOSTNAME_RESOLUTION_TIMEOUT: Duration = Duration::from_secs(10);

async fn try_resolve_hostname(hostname: &str) -> Result<Vec<IpAddr>> {
    tracing::debug!("Trying to resolve hostname: {hostname}");
    let resolver = HickoryDnsResolver::default();

    let addrs =
        match tokio::time::timeout(HOSTNAME_RESOLUTION_TIMEOUT, resolver.resolve_str(hostname))
            .await
        {
            Ok(Ok(addrs)) => addrs,
            Ok(Err(err)) => {
                trace_err_chain!(err, "Failed to resolve hostname");
                return Err(Error::FailedToDnsResolveGateway {
                    hostname: hostname.to_string(),
                    source: err,
                });
            }
            Err(_timeout) => {
                return Err(Error::HostnameResolutionTimeout {
                    hostname: hostname.to_string(),
                });
            }
        };

    tracing::debug!("Resolved to: {addrs:?}");

    let ips = addrs.iter().collect::<Vec<_>>();
    if ips.is_empty() {
        return Err(Error::ResolvedHostnameButNoIp(hostname.to_string()));
    }

    Ok(ips)
}

async fn url_to_socket_addr(unresolved_url: &url::Url) -> Result<Vec<SocketAddr>> {
    let port = unresolved_url
        .port_or_known_default()
        .ok_or(Error::UrlError {
            url: unresolved_url.clone(),
            reason: "missing port".to_string(),
        })?;
    let hostname = unresolved_url.host_str().ok_or(Error::UrlError {
        url: unresolved_url.clone(),
        reason: "missing hostname".to_string(),
    })?;

    Ok(try_resolve_hostname(hostname)
        .await?
        .into_iter()
        .map(|ip| SocketAddr::new(ip, port))
        .collect())
}

pub async fn resolve_config(config: &Config) -> Result<ResolvedConfig> {
    let nyxd_socket_addrs = url_to_socket_addr(config.nyxd_url()).await?;
    let api_socket_addrs = url_to_socket_addr(config.api_url()).await?;
    let nym_vpn_api_socket_addrs = if let Some(vpn_api_url) = config.nym_vpn_api_url() {
        Some(url_to_socket_addr(vpn_api_url).await?)
    } else {
        None
    };

    Ok(ResolvedConfig {
        nyxd_socket_addrs,
        api_socket_addrs,
        nym_vpn_api_socket_addrs,
    })
}

pub fn split_ips(ips: Vec<IpAddr>) -> (Vec<Ipv4Addr>, Vec<Ipv6Addr>) {
    ips.into_iter()
        .fold((vec![], vec![]), |(mut v4, mut v6), ip| {
            match ip {
                IpAddr::V4(ipv4_addr) => v4.push(ipv4_addr),
                IpAddr::V6(ipv6_addr) => v6.push(ipv6_addr),
            }
            (v4, v6)
        })
}
