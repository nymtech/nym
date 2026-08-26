// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::*;
use hickory_resolver::TokioResolver;
use itertools::Itertools;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

/// IP addresses guaranteed to fail attempts to resolve
///
/// Addresses drawn from blocks set off by RFC5737 (ipv4) and RFC3849 (ipv6)
const GUARANTEED_BROKEN_IPS_1: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)),
    IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)),
    IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0x1111)),
    IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0x1001)),
];

#[tokio::test]
async fn reqwest_with_custom_dns() {
    let var_name = HickoryDnsResolver::new();
    let resolver = var_name;
    let client = reqwest::ClientBuilder::new()
        .dns_resolver(resolver)
        .build()
        .unwrap();

    let resp = client
        .get("http://ifconfig.me:80")
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    assert!(!resp.is_empty());
}

#[tokio::test]
async fn dns_lookup() -> Result<(), ResolveError> {
    let resolver = HickoryDnsResolver::new();

    let domain = "ifconfig.me";
    let addrs = resolver.resolve_str(domain).await?;

    assert!(addrs.into_iter().next().is_some());

    Ok(())
}

#[tokio::test]
async fn static_resolver_as_fallback() -> Result<(), ResolveError> {
    let example_domain = "non-existent.nymvpn.com";
    let mut resolver: HickoryDnsResolver = HickoryDnsResolver {
        use_shared: false,
        ..Default::default()
    };

    let result = resolver.resolve_str(example_domain).await;
    assert!(result.is_err()); // should be NXDomain

    resolver.static_base = Some(Default::default());

    let mut addr_map = HashMap::new();
    let example_ip4: IpAddr = "10.10.10.10".parse().unwrap();
    let example_ip6: IpAddr = "dead::beef".parse().unwrap();
    addr_map.insert(example_domain.to_string(), vec![example_ip4, example_ip6]);

    resolver.set_fallback_addrs(addr_map);

    let addrs = resolver.resolve_str(example_domain).await?.collect_vec();
    assert!(addrs.contains(&example_ip4));
    assert!(addrs.contains(&example_ip6));
    Ok(())
}

/// Resetting the nameserver group on an independent (non-shared) resolver forces the internal
/// resolver to be rebuilt: a lookup that succeeded against the original (real) nameservers
/// fails once the group is swapped for one that is guaranteed to be unreachable.
#[tokio::test]
async fn set_name_servers_rebuilds_independent_resolver() -> Result<(), ResolveError> {
    let resolver: HickoryDnsResolver = HickoryDnsResolver {
        use_shared: false,
        overall_dns_timeout: Duration::from_secs(5),
        ..Default::default()
    };

    // sanity: the default nameservers can resolve a real domain, and get_name_servers()
    // reflects the default group before any override.
    assert_eq!(
        resolver
            .get_name_servers()
            .iter()
            .map(|ns| ns.ip)
            .collect::<Vec<_>>(),
        default_nameserver_group_ipv4_only()
            .iter()
            .map(|ns| ns.ip)
            .collect::<Vec<_>>()
    );
    assert!(resolver.resolve_str("ifconfig.me").await?.next().is_some());

    let broken_domain = Arc::<str>::from("cloudflare-dns.com");
    let broken_ns = GUARANTEED_BROKEN_IPS_1
        .iter()
        .map(|ip| NameServerConfig::tls(*ip, broken_domain.clone()))
        .collect::<Vec<_>>();
    resolver.set_name_servers(broken_ns.clone());
    assert_eq!(
        resolver
            .get_name_servers()
            .iter()
            .map(|ns| ns.ip)
            .collect::<Vec<_>>(),
        broken_ns.iter().map(|ns| ns.ip).collect::<Vec<_>>()
    );

    // a fresh (uncached) lookup now fails, proving the internal resolver was rebuilt against
    // the new nameserver group rather than reusing the previously cached, working one.
    let result = resolver.resolve_str("non-existent.nymtech.net").await;
    assert!(result.is_err_and(|e| e.is_timeout()));

    Ok(())
}

/// Resetting the nameserver group through a resolver backed by the shared resolver propagates
/// to the shared resolver itself: any other instance backed by the shared resolver observes
/// the new (broken) nameservers too, even one created after the change.
#[tokio::test]
#[allow(clippy::await_holding_lock)] // guard is only ever held on the single-threaded test runtime
async fn set_name_servers_on_shared_resolver() -> Result<(), ResolveError> {
    // mutates the process-wide shared resolver's nameserver group and cached resolver, so must
    // not run concurrently with any other test that resolves through the shared resolver.
    let _guard = crate::lock_shared_test_state();

    let default_ns = default_nameserver_group_ipv4_only();

    let resolver1 = HickoryDnsResolver::new();
    // sanity: the shared resolver can resolve a real domain before the reset.
    assert!(resolver1.resolve_str("ifconfig.me").await?.next().is_some());

    let broken_domain = Arc::<str>::from("cloudflare-dns.com");
    let broken_ns = GUARANTEED_BROKEN_IPS_1
        .iter()
        .map(|ip| NameServerConfig::tls(*ip, broken_domain.clone()))
        .collect::<Vec<_>>();
    resolver1.set_name_servers(broken_ns);

    // a different, freshly created instance backed by the shared resolver must also observe
    // the change, since it lazily builds off of the shared (now-broken) nameserver group.
    let resolver2: HickoryDnsResolver = HickoryDnsResolver {
        overall_dns_timeout: Duration::from_secs(5),
        ..Default::default()
    };
    let result = resolver2.resolve_str("non-existent.nymtech.net").await;

    // restore the shared resolver so later tests aren't affected by this one, regardless of
    // the assertion outcome below.
    resolver1.set_name_servers(default_ns);

    assert!(result.is_err_and(|e| e.is_timeout()));

    Ok(())
}

#[test]
fn edge1_streaming_gateway_com_is_pinned_to_live_ipv4() {
    let addrs = constants::default_static_addrs();
    let pinned = addrs
        .get(constants::NYM_VPN_API_EDGE1_STREAMING_GATEWAY_COM)
        .expect("edge1.streaming-gateway.com must be statically pinned after smoke");
    assert_eq!(
        pinned,
        &constants::NYM_VPN_API_EDGE1_STREAMING_GATEWAY_COM_IPS.to_vec()
    );
    assert_eq!(pinned, &vec![IpAddr::V4(Ipv4Addr::new(139, 162, 57, 231))]);
}

// Test the nameserver trial functionality with mostly nameservers guaranteed to be broken and
// one that should work.
#[tokio::test]
async fn trial_nameservers() {
    let good_cf_ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));

    let mut ns_ips = GUARANTEED_BROKEN_IPS_1.to_vec();
    ns_ips.push(good_cf_ip);

    let domain = Arc::<str>::from("cloudflare-dns.com");
    let path = Arc::<str>::from("/dns-query");
    let broken_ns_https = GUARANTEED_BROKEN_IPS_1
        .iter()
        .chain([&good_cf_ip])
        .map(|ip| NameServerConfig::https(*ip, domain.clone(), Some(path.clone())))
        .collect::<Vec<_>>();

    for (ns, result) in crate::dns::trial::trial_nameservers_inner(&broken_ns_https).await {
        if ns.ip == good_cf_ip {
            assert!(result.is_ok())
        } else {
            assert!(result.is_err())
        }
    }
}

mod failure_test {
    use super::*;

    // Create a resolver that behaves the same as the custom configured router, except for the fact
    // that it is guaranteed to fail.
    fn build_broken_resolver() -> Result<TokioResolver, ResolveError> {
        info!("building new faulty resolver");

        let domain = Arc::<str>::from("cloudflare-dns.com");
        let path = Arc::<str>::from("/dns-query");
        let broken_ns_group = GUARANTEED_BROKEN_IPS_1
            .iter()
            .map(|ip| NameServerConfig::tls(*ip, domain.clone()))
            .chain(
                GUARANTEED_BROKEN_IPS_1
                    .iter()
                    .map(|ip| NameServerConfig::https(*ip, domain.clone(), Some(path.clone())))
                    .collect::<Vec<_>>(),
            )
            .collect::<Vec<_>>();

        configure_and_build_resolver(broken_ns_group)
    }

    #[tokio::test]
    async fn dns_lookup_failures() -> Result<(), ResolveError> {
        let time_start = std::time::Instant::now();

        let r = OnceCell::new();
        r.set(build_broken_resolver().expect("failed to build resolver"))
            .expect("broken resolver init error");

        // create a new resolver that won't mess with the shared resolver used by other tests
        let resolver = HickoryDnsResolver {
            use_shared: false,
            state: Arc::new(ArcSwap::from_pointee(r)),
            overall_dns_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        build_broken_resolver()?;
        let domain = "ifconfig.me";
        let result = resolver.resolve_str(domain).await;
        assert!(result.is_err_and(|e| e.is_timeout()));

        let duration = time_start.elapsed();
        assert!(duration < resolver.overall_dns_timeout + Duration::from_secs(1));

        Ok(())
    }

    #[tokio::test]
    async fn fallback_to_static() -> Result<(), ResolveError> {
        let r = OnceCell::new();
        r.set(build_broken_resolver().expect("failed to build resolver"))
            .expect("broken resolver init error");

        // create a new resolver that won't mess with the shared resolver used by other tests
        let resolver = HickoryDnsResolver {
            use_shared: false,
            state: Arc::new(ArcSwap::from_pointee(r)),
            static_base: Some(Default::default()),
            overall_dns_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        build_broken_resolver()?;

        // successful lookup using fallback to static resolver
        let domain = "nymvpn.com";
        let _ = resolver
            .resolve_str(domain)
            .await
            .expect("failed to resolve address in static lookup");

        // unsuccessful lookup - primary times out, and not in static table
        let domain = "non-existent.nymtech.net";
        let result = resolver.resolve_str(domain).await;
        assert!(result.is_err_and(|e| e.is_timeout()));

        Ok(())
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // guard is only ever held on the single-threaded test runtime
    async fn setting_dns_fallbacks_with_shared_resolver() -> Result<(), ResolveError> {
        // mutates the process-wide shared resolver's nameserver group and cached resolver, so must
        // not run concurrently with any other test that resolves through the shared resolver.
        let _guard = crate::lock_shared_test_state();

        let resolver1 = HickoryDnsResolver::new();

        // create a new resolver that uses the shared resolver
        let mut resolver = HickoryDnsResolver::new();

        let example_domains = [
            String::from("static1.nymvpn.com"),
            String::from("static2.nymvpn.com"),
        ];
        let mut addr_map1 = HashMap::new();
        addr_map1.insert(
            example_domains[0].clone(),
            vec![Ipv4Addr::new(10, 10, 10, 10).into()],
        );
        addr_map1.insert(
            example_domains[1].clone(),
            vec![Ipv4Addr::new(1, 1, 1, 1).into()],
        );

        resolver.set_static_preresolve(addr_map1);

        let time_start = std::time::Instant::now();
        // successful lookup using pre-resolve entry promoted from fallback
        let _ = resolver1
            .resolve_str(&example_domains[0])
            .await
            .expect("domain expected to be in pre-resolve");

        // this lookup should basically be instant as we are using pre-resolve
        let lookup_dur = std::time::Instant::now() - time_start;
        assert!(
            lookup_dur < Duration::from_millis(10),
            "expected instant - took {}ms",
            (lookup_dur).as_millis()
        );

        // After clearing the pre-resolve in one instance of the shared resolver ...
        resolver.clear_preresolve();

        // ... other instances have their pre-resolve entries cleared.
        let prereslve_lookup = resolver1
            .static_base
            .as_ref()
            .unwrap()
            .get()
            .unwrap()
            .pre_resolve(&example_domains[0]);
        assert!(prereslve_lookup.is_none());

        // cleanup state of shared resolver before finishing test
        resolver1.clear_preresolve();

        Ok(())
    }

    #[tokio::test]
    #[cfg(any())] // #[ignore] we run --ignore in CI/CD assuming it just means slow -_-
    // This test impacts the state of the shared resolver and as such is disabled to avoid
    // interference with other tests.
    //
    // this test is dependent on external network setup -- i.e. blocking all traffic to the
    // default resolvers. Otherwise the default resolvers will succeed without using the static
    // fallback, making the test pointless
    async fn dns_lookup_failure_on_shared() -> Result<(), ResolveError> {
        let resolver1 = HickoryDnsResolver::shared();

        let time_start = std::time::Instant::now();
        // create a new resolver that uses the shared resolver
        let resolver = HickoryDnsResolver::shared();

        // successful lookup using fallback to static resolver
        let domain = "rpc.nymtech.net";
        let _ = resolver
            .resolve_str(domain)
            .await
            .expect("failed to resolve address in static lookup");

        let lookup_dur = Instant::now() - time_start;
        assert!(
            lookup_dur > resolver.overall_dns_timeout,
            "expected lookup timeout - took {}ms",
            (lookup_dur).as_millis()
        );

        let time_start = std::time::Instant::now();
        // successful lookup using pre-resolve entry promoted from fallback
        let domain = "rpc.nymtech.net";
        let _ = resolver1
            .resolve_str(domain)
            .await
            .expect("domain expected to be in pre-resolve");

        // this lookup should basically be instant as we are using pre-resolve
        let lookup_dur = std::time::Instant::now() - time_start;
        assert!(
            lookup_dur < Duration::from_millis(10),
            "expected instant - took {}ms",
            (lookup_dur).as_millis()
        );

        // unsuccessful lookup - primary times out, and not in static table
        let domain = "non-existent.nymtech.net";
        let result = resolver.resolve_str(domain).await;
        assert!(result.is_err());
        // assert!(result.is_err_and(|e| matches!(e, ResolveError::Timeout)));
        // assert!(result.is_err_and(|e| matches!(e, ResolveError::ResolveError(e) if e.is_nx_domain())));
        Ok(())
    }
}
