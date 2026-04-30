use crate::dns::ResolveError;

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tracing::*;

const DEFAULT_PRE_RESOLVE_TIMEOUT: Duration = super::DEFAULT_POSITIVE_LOOKUP_CACHE_TTL;

#[derive(Debug, Default, Clone)]
pub struct StaticResolver {
    fallback_addr_map: Arc<Mutex<HashMap<String, Vec<IpAddr>>>>,
    preresolve_addr_map: Arc<Mutex<HashMap<String, Entry>>>,
    pre_resolve_timeout: Option<Duration>,
}

#[derive(Debug, Clone, Default)]
enum PreResolveStatus {
    #[default]
    Valid,
    ValidUntil(Instant),
}

#[derive(Debug, Clone, Default)]
struct Entry {
    status: PreResolveStatus,
    addrs: Vec<IpAddr>,
}

impl Entry {
    fn new(addrs: Vec<IpAddr>) -> Self {
        Self {
            status: PreResolveStatus::Valid,
            addrs,
        }
    }

    fn new_timeout(addrs: Vec<IpAddr>, timeout: Duration) -> Self {
        Self {
            status: PreResolveStatus::ValidUntil(Instant::now() + timeout),
            addrs,
        }
    }

    fn is_valid(&self) -> bool {
        match self.status {
            PreResolveStatus::Valid => true,
            PreResolveStatus::ValidUntil(t) => t > Instant::now(),
        }
    }
}

impl StaticResolver {
    pub fn new() -> StaticResolver {
        Self {
            fallback_addr_map: Arc::new(Mutex::new(HashMap::new())),
            preresolve_addr_map: Arc::new(Mutex::new(HashMap::new())),
            pre_resolve_timeout: Some(DEFAULT_PRE_RESOLVE_TIMEOUT),
        }
    }

    /// Initialize the contents of the pre-resolve table for this instance of the static resolver
    #[allow(unused)]
    pub fn with_preresolve(mut self, entries: HashMap<String, Vec<IpAddr>>) -> Self {
        let entries = entries
            .into_iter()
            .map(|(name, ips)| (name, Entry::new(ips)))
            .collect();
        self.preresolve_addr_map = Arc::new(Mutex::new(entries));
        self
    }

    /// Initialize the contenes of the fallback table for this instance of the static resolver
    pub fn with_fallback(mut self, entries: HashMap<String, Vec<IpAddr>>) -> Self {
        self.fallback_addr_map = Arc::new(Mutex::new(entries));
        self
    }

    /// Return the set of domain names and associated addresses stored in the pre-resolve static
    /// lookup table
    pub fn get_preresolve_addrs(&self) -> HashMap<String, Vec<IpAddr>> {
        let mut out = HashMap::new();
        self.preresolve_addr_map
            .lock()
            .unwrap()
            .iter()
            .for_each(|(name, entry)| {
                out.insert(name.clone(), entry.addrs.clone());
            });
        out
    }

    /// Return the set of domain names and associated addresses stored in the fallback static lookup
    /// table
    pub fn get_fallback_addrs(&self) -> HashMap<String, Vec<IpAddr>> {
        self.fallback_addr_map.lock().unwrap().clone()
    }

    /// Set (or overwrite) the map of static addresses to be returned only after attempting a lookup
    /// over the network resolver.
    pub fn set_fallback(&self, addrs: HashMap<String, Vec<IpAddr>>) {
        self.fallback_addr_map.lock().unwrap().extend(addrs);
    }

    /// Clear entries from the static table that would return entries during the pre-resolve stage.
    /// This means that all lookups will attempt to use the network resolver again before the static
    /// table is consulted.
    ///  
    /// Entries elevated to pre-resolve from fallback (added from default or using
    /// [`set_fallback`]`) will have their cache timeout cleared. Entries added directly to
    /// pre-resolve (using [`Self::preresolve_to_addrs`]) will be removed.
    pub fn clear_preresolve(&self) {
        *self.preresolve_addr_map.lock().unwrap() = HashMap::new();
    }

    /// Set (or overwrite) the map of static addresses and mark these domains to be returned
    /// WITHOUT attempting a lookup over the network resolver.
    pub fn set_preresolve(&self, addrs: HashMap<String, Vec<IpAddr>>) {
        let mut current_map = self.preresolve_addr_map.lock().unwrap();
        for (domain, ips) in addrs.into_iter() {
            _ = current_map.insert(domain, Entry::new(ips))
        }
    }

    /// Change the timeout for which domains can be pre-resolved after they are looked up in the
    /// static lookup table.
    #[allow(unused)]
    pub fn with_pre_resolve_timeout(mut self, timeout: Duration) -> Self {
        self.pre_resolve_timeout = Some(timeout);
        self
    }

    /// Try looking up the domain in the static table. If the domain is in the table AND  we have
    /// recently (within the configured timeout) looked it up previously in this static table using
    /// a regular resolve.
    pub fn pre_resolve(&self, name: &str) -> Option<Vec<IpAddr>> {
        self.preresolve_addr_map
            .lock()
            .unwrap()
            .get(name)
            .filter(|entry| entry.is_valid())
            .map(|entry| {
                debug!("pre-resolve lookup hit for \"{name:?}\" in static table resolver");
                entry.addrs.clone()
            })
    }

    #[allow(unused)]
    pub fn resolve_str(&self, name: &str) -> Option<Vec<IpAddr>> {
        Self::resolve_inner(
            self.fallback_addr_map.lock().unwrap(),
            self.preresolve_addr_map.lock().unwrap(),
            name,
            self.pre_resolve_timeout,
        )
    }

    fn resolve_inner(
        fallback_table: MutexGuard<'_, HashMap<String, Vec<IpAddr>>>,
        mut preresolve_table: MutexGuard<'_, HashMap<String, Entry>>,
        name: &str,
        pre_resolve_cache_timeout: Option<Duration>,
    ) -> Option<Vec<IpAddr>> {
        let resolved = fallback_table.get(name)?;

        debug!("lookup hit for \"{name:?}\" in static table resolver");

        // We had to look this entry up and a pre-resolve duration is defined, so it will
        // trigger in pre-resolve lookups for the next _timeout_ window if it wasn't already
        // triggering.
        if let Some(pre_resolve_timeout) = pre_resolve_cache_timeout {
            match preresolve_table.get_mut(name) {
                None => {
                    _ = preresolve_table.insert(
                        name.to_string(),
                        Entry::new_timeout(resolved.clone(), pre_resolve_timeout),
                    );
                }
                // Not sure how we would get cases where this is Some( ) -- it requires having a
                // Valid entry in the preresolve table and still doing a lookup against fallback.
                Some(entry) if matches!(entry.status, PreResolveStatus::ValidUntil(_)) => {
                    _ = preresolve_table.insert(
                        name.to_string(),
                        Entry::new_timeout(resolved.clone(), pre_resolve_timeout),
                    );
                }
                _ => {}
            }
        }
        Some(resolved.clone())
    }
}

impl Resolve for StaticResolver {
    fn resolve(&self, name: Name) -> Resolving {
        debug!("looking up {name:?} in static resolver");
        // these should clone arcs, not the actual tables
        let fallback_addr_map = self.fallback_addr_map.clone();
        let presesolve_addr_map = self.preresolve_addr_map.clone();
        let timeout = self.pre_resolve_timeout;
        // Also the returned future doesn't try to take the lock on the tables until the
        // future is awaited, so no blocking issues.
        Box::pin(async move {
            let fallback_addr_map = fallback_addr_map.lock().unwrap();
            let presesolve_addr_map = presesolve_addr_map.lock().unwrap();
            let lookup = match Self::resolve_inner(
                fallback_addr_map,
                presesolve_addr_map,
                name.as_str(),
                timeout,
            ) {
                None => return Err(ResolveError::StaticLookupMiss.into()),
                Some(addrs) => addrs,
            };
            let addrs: Addrs = Box::new(
                lookup
                    .into_iter()
                    .map(|ip_addr| SocketAddr::new(ip_addr, 0)),
            );

            Ok(addrs)
        })
    }
}

#[cfg(test)]
mod test {
    use itertools::Itertools;

    use super::*;
    use std::error::Error as StdError;
    use std::net::Ipv4Addr;
    use std::str::FromStr;

    #[tokio::test]
    async fn lookup_using_static_resolver() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let example_domain = String::from("static.nymvpn.com");

        // lookup for domain for which there is no entry
        let resolver = StaticResolver::new();

        let url = reqwest::dns::Name::from_str(&example_domain).unwrap();
        let result = resolver.resolve(url).await;
        assert!(result.is_err());
        match result {
            Ok(_) => panic!("lookup with empty map should fail"),
            Err(e) => assert_eq!(e.to_string(), ResolveError::StaticLookupMiss.to_string()),
        }

        // Successful lookup
        let mut addr_map = HashMap::new();
        let example_ip4: IpAddr = "10.10.10.10".parse().unwrap();
        let example_ip6: IpAddr = "dead::beef".parse().unwrap();
        addr_map.insert(example_domain.clone(), vec![example_ip4, example_ip6]);

        let url = reqwest::dns::Name::from_str(&example_domain).unwrap();
        let resolver = StaticResolver::new().with_fallback(addr_map);
        let mut addrs = resolver.resolve(url).await?;
        assert!(addrs.contains(&SocketAddr::new(example_ip4, 0)));
        assert!(addrs.contains(&SocketAddr::new(example_ip6, 0)));

        Ok(())
    }

    #[test]
    fn elevate_fallback_to_pre_resolve() {
        let example_duration = Duration::from_secs(3);
        let example_domain = String::from("static.nymvpn.com");
        let mut addr_map = HashMap::new();
        let example_ip4: IpAddr = "10.10.10.10".parse().unwrap();
        let example_ip6: IpAddr = "dead::beef".parse().unwrap();
        addr_map.insert(example_domain.clone(), vec![example_ip4, example_ip6]);

        let resolver = StaticResolver::new()
            .with_fallback(addr_map)
            .with_pre_resolve_timeout(example_duration);

        // ensure that attempting to pre-resolve without first resolving returns none
        let result = resolver.pre_resolve(&example_domain);
        assert!(result.is_none());

        // resolving should now update the pre-resolve validity timeout for the entry
        let _addrs = resolver
            .resolve_str(&example_domain)
            .expect("entry should exist");
        assert!(matches!(
            resolver.preresolve_status(&example_domain),
            Some(PreResolveStatus::ValidUntil(t))
                if t < Instant::now() + example_duration
        ));

        // check that pre-resolve now returns the expected record
        let addrs = resolver
            .pre_resolve(&example_domain)
            .expect("entry should be in pre-resolve now");
        assert!(addrs.contains(&example_ip4));

        std::thread::sleep(example_duration);

        // check that after the timeout duration the pre-resolve no longer returns the address
        let result = resolver.pre_resolve(&example_domain);
        assert!(result.is_none());
    }

    #[test]
    fn set_and_use_preresolve() {
        let example_duration = Duration::from_secs(3);
        let example_domains = [
            String::from("static1.nymvpn.com"),
            String::from("static2.nymvpn.com"),
            String::from("preresolve.nymvpn.com"),
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

        let mut addr_map2 = HashMap::new();
        addr_map2.insert(
            example_domains[1].clone(),
            vec![Ipv4Addr::new(1, 1, 1, 1).into()],
        );
        addr_map2.insert(
            example_domains[2].clone(),
            vec![Ipv4Addr::new(8, 8, 8, 8).into()],
        );

        let resolver = StaticResolver::new()
            .with_fallback(addr_map1)
            .with_pre_resolve_timeout(example_duration);

        // Attempting to pre-resolve without setting the table returns none
        let result = resolver.pre_resolve(&example_domains[0]);
        assert!(result.is_none());

        resolver.set_preresolve(addr_map2);

        // After setting the pre-resolve, addresses in the the table are returned
        let result = resolver.pre_resolve(&example_domains[1]);
        assert!(result.is_some());

        // If the domain wasn't in the pre-resolve table it returns none.
        let result = resolver.pre_resolve(&example_domains[0]);
        assert!(result.is_none());

        resolver.clear_preresolve();
    }

    #[test]
    fn preresolve_with_fallback() {
        let example_duration = Duration::from_secs(3);
        let example_domains = [
            String::from("static1.nymvpn.com"),
            String::from("static2.nymvpn.com"),
            String::from("preresolve.nymvpn.com"),
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

        let mut addr_map2 = HashMap::new();
        addr_map2.insert(
            example_domains[1].clone(),
            vec![Ipv4Addr::new(1, 1, 1, 1).into()],
        );
        addr_map2.insert(
            example_domains[2].clone(),
            vec![Ipv4Addr::new(8, 8, 8, 8).into()],
        );

        let resolver = StaticResolver::new()
            .with_fallback(addr_map1)
            .with_preresolve(addr_map2)
            .with_pre_resolve_timeout(example_duration);

        // when using both pre-resolve and fallback elevating entries from fallback to pre-resolve
        // leaves the entries as `Valid`.
        assert!(matches!(
            resolver.preresolve_status(&example_domains[1]),
            Some(PreResolveStatus::Valid)
        ));
        let _addrs = resolver
            .resolve_str(&example_domains[1])
            .expect("entry should exist");
        assert!(matches!(
            resolver.preresolve_status(&example_domains[1]),
            Some(PreResolveStatus::Valid)
        ));

        // entries not already in pre-resolve get elevated with a timeout.
        assert!(!resolver.preresolve_contains(&example_domains[0]));
        let _addrs = resolver
            .resolve_str(&example_domains[0])
            .expect("entry should exist");
        assert!(resolver.preresolve_contains(&example_domains[0]));
        assert!(matches!(
            resolver.preresolve_status(&example_domains[0]),
            Some(PreResolveStatus::ValidUntil(_))
        ));

        // clearing the pre-resolve table doesn't impact the fallback table.
        resolver.clear_preresolve();
        assert!(!resolver.preresolve_contains(&example_domains[0]));
        assert!(!resolver.preresolve_contains(&example_domains[1]));
        assert!(!resolver.preresolve_contains(&example_domains[2]));
        assert!(!resolver.fallback_contains(&example_domains[0]));
        assert!(!resolver.fallback_contains(&example_domains[1]));
    }

    /// convenience functions for testing
    impl StaticResolver {
        fn preresolve_status(&self, name: &str) -> Option<PreResolveStatus> {
            self.preresolve_addr_map
                .lock()
                .unwrap()
                .get(name)
                .map(|e| e.status.clone())
        }

        fn preresolve_contains(&self, name: &str) -> bool {
            self.preresolve_addr_map.lock().unwrap().contains_key(name)
        }

        fn fallback_contains(&self, name: &str) -> bool {
            self.preresolve_addr_map.lock().unwrap().contains_key(name)
        }
    }
}
