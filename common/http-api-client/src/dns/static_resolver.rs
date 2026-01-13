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
    static_addr_map: Arc<Mutex<HashMap<String, Entry>>>,
    pre_resolve_timeout: Option<Duration>,
}

#[derive(Debug, Clone, Default)]
enum PreResolveStatus {
    Valid,
    ValidUntil(Instant),
    #[default]
    Invalid,
}

#[derive(Debug, Clone, Default)]
struct Entry {
    status: PreResolveStatus,
    addrs: Vec<IpAddr>,
}

impl Entry {
    fn new(addrs: Vec<IpAddr>) -> Self {
        Self {
            status: PreResolveStatus::Invalid,
            addrs,
        }
    }

    fn is_valid(&self) -> bool {
        match self.status {
            PreResolveStatus::Invalid => false,
            PreResolveStatus::Valid => true,
            PreResolveStatus::ValidUntil(t) => t > Instant::now(),
        }
    }
}

impl StaticResolver {
    pub fn new(static_entries: HashMap<String, Vec<IpAddr>>) -> StaticResolver {
        debug!("building static resolver");
        let static_entries = static_entries
            .into_iter()
            .map(|(name, ips)| (name, Entry::new(ips)))
            .collect();
        Self {
            static_addr_map: Arc::new(Mutex::new(static_entries)),
            pre_resolve_timeout: Some(DEFAULT_PRE_RESOLVE_TIMEOUT),
        }
    }

    /// Return the full set of domain names and associated addresses stored in this static lookup table
    pub fn get_addrs(&self) -> HashMap<String, Vec<IpAddr>> {
        let mut out = HashMap::new();
        self.static_addr_map
            .lock()
            .unwrap()
            .iter()
            .for_each(|(name, entry)| {
                out.insert(name.clone(), entry.addrs.clone());
            });
        out
    }

    /// Clear entries from the static table that would return entries during the pre-resolve stage.
    /// This means that all lookups will attempt to use the network resolver again before the static
    /// table is consulted.
    ///  
    /// Entries elevated to pre-resolve from fallback (added from default or using
    /// [`fallback_to_addrs`]`) will have their cache timeout cleared. Entries added directly to
    /// pre-resolve (using [`Self::preresolve_to_addrs`]) will be removed.
    ///
    /// (Corner case) entries that were added first as fallback, then overwritten with pre-resolve
    /// entries using [`Self::preresolve_to_addrs`] will not be downgraded back to fallback. They
    /// will be removed like all other pre-resolve entries.
    pub fn clear_preresolve(&self) {
        let mut to_remove = Vec::new();
        let mut current_map = self.static_addr_map.lock().unwrap();
        for (domain, entry) in current_map.iter_mut() {
            match entry.status {
                // retain entries that are there for static fallback
                PreResolveStatus::Invalid => {}
                // clear pre-resolve cache timeout for entries elevated from fallback
                PreResolveStatus::ValidUntil(_) => entry.status = PreResolveStatus::Invalid,
                // remove entries added exclusively for pre-resolve
                PreResolveStatus::Valid => to_remove.push(domain.clone()),
            }
        }
        to_remove.iter().for_each(|k| {
            _ = current_map.remove(k);
        });
    }

    /// Set (or overwrite) the map of static addresses and mark these domains to be returned
    /// WITHOUT attempting a lookup over the network resolver.
    pub fn preresolve_to_addrs(&self, addrs: HashMap<String, Vec<IpAddr>>) {
        let mut current_map = self.static_addr_map.lock().unwrap();
        for (domain, ips) in addrs.into_iter() {
            _ = current_map.insert(
                domain,
                Entry {
                    status: PreResolveStatus::Valid,
                    addrs: ips,
                },
            )
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
        debug!("found {name:?} in pre-resolve static table resolver");

        self.pre_resolve_timeout?;

        self.static_addr_map
            .lock()
            .unwrap()
            .get(name)
            .filter(|e| e.is_valid())
            .map(|e| e.addrs.clone())
    }

    #[allow(unused)]
    pub fn resolve_str(&self, name: &str) -> Option<Vec<IpAddr>> {
        Self::resolve_inner(
            self.static_addr_map.lock().unwrap(),
            name,
            self.pre_resolve_timeout,
        )
        .map(|e| e.addrs)
    }

    fn resolve_inner(
        mut table: MutexGuard<'_, HashMap<String, Entry>>,
        name: &str,
        pre_resolve_cache_timeout: Option<Duration>,
    ) -> Option<Entry> {
        let resolved = table.get_mut(name)?;

        debug!("found {name:?} in static table resolver");

        // We had to look this entry up and a pre-resolve duration is defined, so it will
        // trigger in pre-resolve lookups for the next _timeout_ window if it wasn't already
        // triggering.
        if let Some(pre_resolve_timeout) = pre_resolve_cache_timeout {
            let timeout = Instant::now() + pre_resolve_timeout;
            match resolved.status {
                PreResolveStatus::Invalid => {
                    resolved.status = PreResolveStatus::ValidUntil(timeout)
                }
                PreResolveStatus::ValidUntil(t) => {
                    if t < timeout {
                        resolved.status = PreResolveStatus::ValidUntil(timeout)
                    }
                }
                PreResolveStatus::Valid => {}
            }
        }
        Some(resolved.clone())
    }
}

impl Resolve for StaticResolver {
    fn resolve(&self, name: Name) -> Resolving {
        debug!("looking up {name:?} in static resolver");
        let addr_map = self.static_addr_map.clone();
        let timeout = self.pre_resolve_timeout;
        Box::pin(async move {
            let addr_map = addr_map.lock().unwrap();
            let lookup = match Self::resolve_inner(addr_map, name.as_str(), timeout) {
                None => return Err(ResolveError::StaticLookupMiss.into()),
                Some(entry) => entry.addrs,
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
        let resolver = StaticResolver::new(HashMap::new());

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
        let resolver = StaticResolver::new(addr_map);
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

        let resolver = StaticResolver::new(addr_map).with_pre_resolve_timeout(example_duration);

        // ensure that attempting to pre-resolve without first resolving returns none
        let result = resolver.pre_resolve(&example_domain);
        assert!(result.is_none());

        // resolving should now update the pre-resolve validity timeout for the entry
        let entry = StaticResolver::resolve_inner(
            resolver.static_addr_map.lock().unwrap(),
            &example_domain,
            Some(example_duration),
        )
        .expect("missing entry???!!!!");
        assert!(matches!(entry
                .status, PreResolveStatus::ValidUntil(t) if t < Instant::now() + example_duration));

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
        let example_domains = vec![
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

        let resolver = StaticResolver::new(addr_map1).with_pre_resolve_timeout(example_duration);

        // ensure that attempting to pre-resolve without first resolving returns none
        let result = resolver.pre_resolve(&example_domains[0]);
        assert!(result.is_none());

        resolver.preresolve_to_addrs(addr_map2);

        // ensure that attempting to pre-resolve without first resolving returns none
        let result = resolver.pre_resolve(&example_domains[0]);
        assert!(result.is_none());

        // ensure that attempting to pre-resolve without first resolving returns none
        let result = resolver.pre_resolve(&example_domains[1]);
        assert!(result.is_some());

        let result = resolver.pre_resolve(&example_domains[2]);
        assert!(result.is_some());

        resolver.clear_preresolve();

        println!("{:?}", resolver.get_addrs());
    }
}
