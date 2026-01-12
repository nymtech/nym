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
struct Entry {
    valid_for_pre_resolve_until: Option<Instant>,
    addrs: Vec<IpAddr>,
}

impl Entry {
    fn new(addrs: Vec<IpAddr>) -> Self {
        Self {
            valid_for_pre_resolve_until: None,
            addrs,
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
            .filter(|e| {
                e.valid_for_pre_resolve_until
                    .is_some_and(|t| t > Instant::now())
            })
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
        timeout: Option<Duration>,
    ) -> Option<Entry> {
        let resolved = table.get_mut(name)?;

        debug!("found {name:?} in static table resolver");

        if let Some(pre_resolve_timeout) = timeout {
            // We had to look this entry up and a pre-resolve duration is defined, so it will
            // trigger in pre-resolve lookups for the next _timeout_ window.
            resolved.valid_for_pre_resolve_until = Some(Instant::now() + pre_resolve_timeout);
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
    fn static_lookup_pre_resolve() {
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
        assert!(
            entry
                .valid_for_pre_resolve_until
                .is_some_and(|t| t < Instant::now() + example_duration)
        );

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
}
