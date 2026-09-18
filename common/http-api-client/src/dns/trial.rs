// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dns::*;

use hickory_resolver::{config::NameServerConfig, net::runtime::TokioRuntimeProvider};

impl<C: SharedResolverState> HickoryDnsResolver<C> {
    /// Do a trial resolution using each nameserver individually to test which are working and which
    /// fail to complete a lookup. This will always try the full set of default configured resolvers.
    ///
    /// This diagnostic always runs over Tokio (via [`TokioRuntimeProvider`]), independent of the
    /// connection provider `C` this resolver is otherwise configured with.
    pub async fn trial_nameservers(&self) {
        let nameservers = default_nameserver_group();
        for (ns, result) in trial_nameservers_inner(&nameservers).await {
            if let Err(e) = result {
                warn!("trial {ns:?} errored: {e}");
            } else {
                info!("trial {ns:?} succeeded");
            }
        }
    }
}

/// Do a trial resolution using each nameserver individually to test which are working and which
/// fail to complete a lookup.
pub(crate) async fn trial_nameservers_inner(
    name_servers: &[NameServerConfig],
) -> Vec<(NameServerConfig, Result<(), ResolveError>)> {
    let mut trial_lookups = tokio::task::JoinSet::new();

    for name_server in name_servers {
        let ns = name_server.clone();
        trial_lookups.spawn(async { (ns.clone(), trial_lookup(ns, "example.com").await) });
    }

    trial_lookups.join_all().await
}

/// Create an independent resolver that has only the provided nameserver and do one lookup for the
/// provided query target.
pub(crate) async fn trial_lookup(
    name_server: NameServerConfig,
    query: &str,
) -> Result<(), ResolveError> {
    debug!("running ns trial {name_server:?} query={query}");

    let resolver = configure_and_build_resolver::<TokioRuntimeProvider>(vec![name_server])?;

    match tokio::time::timeout(DEFAULT_OVERALL_LOOKUP_TIMEOUT, resolver.ipv4_lookup(query)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(ResolveError::Timeout),
    }
}
