// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::HostsStore;
use crate::request_filter::allowed_hosts::group::HostsGroup;
use crate::request_filter::allowed_hosts::standard_list::StandardList;
use crate::request_filter::allowed_hosts::stored_allowed_hosts::StoredAllowedHosts;
use std::net::{IpAddr, SocketAddr};
use tokio::sync::Mutex;

#[derive(Debug)]
enum RequestHost {
    IpAddr(IpAddr),
    SocketAddr(SocketAddr),
    RootDomain(String),
}

/// Filters outbound requests based on what's in an `allowed_hosts` list.
///
/// Requests to unknown hosts are automatically written to an `unknown_hosts`
/// list so that they can be copy/pasted into the `allowed_hosts` list if desired.
/// This may be handy for service provider node operators who want to be able to look in the
/// `unknown_hosts` file and allow new hosts (e.g. if a wallet has added a new outbound request
/// which needs to be allowed).
///
/// We rely on the list of domains at https://publicsuffix.org/ to figure out what the root
/// domain is for a given request. This allows us to distinguish all the rules for e.g.
/// .com, .co.uk, .co.jp, uk.com, etc, so that we can distinguish correct root-ish
/// domains as allowed. That list is loaded once at startup from Mozilla's canonical
/// publicsuffix list.
pub(crate) struct OutboundRequestFilter {
    pub(super) allowed_hosts: StoredAllowedHosts,
    pub(super) standard_list: StandardList,
    root_domain_list: publicsuffix::List,
    unknown_hosts: Mutex<HostsStore>,
}

impl OutboundRequestFilter {
    /// Create a new `OutboundRequestFilter` with the given `allowed_hosts` and `unknown_hosts` lists.
    ///
    /// Automatically fetches the latest https://publicsuffix.org/ domain list from the internet so that
    /// the requester can properly parse all of the world's top-level domains in an up-to-date fashion.
    ///
    /// Automatcially fetches the latest standard allowed list from the Nym website, so that all
    /// requesters are able to support the same minimal functionality out of the box.
    pub(crate) fn new(
        allowed_hosts: StoredAllowedHosts,
        standard_list: StandardList,
        unknown_hosts: HostsStore,
    ) -> OutboundRequestFilter {
        let domain_list = match publicsuffix::List::fetch() {
            Ok(list) => list,
            Err(err) => panic!("Couldn't fetch domain list for request filtering, do you have an internet connection?: {err}"),
        };

        OutboundRequestFilter {
            allowed_hosts,
            standard_list,
            root_domain_list: domain_list,
            unknown_hosts: Mutex::new(unknown_hosts),
        }
    }

    pub(crate) fn standard_list(&self) -> StandardList {
        self.standard_list.clone()
    }

    pub(crate) fn allowed_hosts(&self) -> StoredAllowedHosts {
        self.allowed_hosts.clone()
    }

    async fn check_allowed_hosts(&self, host: &RequestHost) -> bool {
        let guard = self.allowed_hosts.get().await;
        self.check_group(&guard.data, host)
    }

    async fn check_standard_list(&self, host: &RequestHost) -> bool {
        let guard = self.standard_list.get().await;
        self.check_group(&guard, host)
    }

    fn check_group(&self, group: &HostsGroup, host: &RequestHost) -> bool {
        match host {
            RequestHost::IpAddr(ip_addr) => group.contains_ip_address(*ip_addr),
            RequestHost::SocketAddr(socket_addr) => group.contains_ip_address(socket_addr.ip()),
            RequestHost::RootDomain(domain) => group.contains_domain(domain),
        }
    }

    async fn add_to_unknown_hosts(&self, host: RequestHost) {
        let mut guard = self.unknown_hosts.lock().await;
        match host {
            RequestHost::IpAddr(ip_addr) => guard.add_ip(ip_addr),
            RequestHost::SocketAddr(socket_addr) => guard.add_ip(socket_addr.ip()),
            RequestHost::RootDomain(domain) => guard.add_domain(&domain),
        }
    }

    fn parse_request_host(&self, host: &str) -> Option<RequestHost> {
        // first check if it's a socket address (ip:port)
        // (this check is performed to not incorrectly strip what we think might be a port
        // from ipv6 address, as for example ::1 contains colons but has no port
        if let Ok(socketaddr) = host.parse::<SocketAddr>() {
            Some(RequestHost::SocketAddr(socketaddr))
            // then check if it was an ip address
        } else if let Ok(ipaddr) = host.parse::<IpAddr>() {
            Some(RequestHost::IpAddr(ipaddr))
            // finally, then assume it might be a domain
        } else {
            // check root
            let trimmed = Self::trim_port(host);
            // if this failed, it was probably some nonsense
            self.get_domain_root(&trimmed).map(RequestHost::RootDomain)
        }
    }

    async fn check_request_host(&self, request_host: &RequestHost) -> bool {
        // first check our own allow list
        let local_allowed = self.check_allowed_hosts(request_host).await;

        // if it's locally allowed, no point in checking the standard list
        if local_allowed {
            return true;
        }

        // if that failed, check the standard list
        self.check_standard_list(request_host).await
    }

    /// Returns `true` if a host's root domain is in the `allowed_hosts` list.
    ///
    /// If it's not in the list, return `false` and write it to the `unknown_hosts` storefile.
    pub(crate) async fn check(&self, host: &str) -> bool {
        let allowed = match self.parse_request_host(host) {
            Some(request_host) => {
                let res = self.check_request_host(&request_host).await;
                if !res {
                    self.add_to_unknown_hosts(request_host).await
                }
                res
            }
            None => false,
        };

        if !allowed {
            log::warn!("Blocked outbound connection to {host}, add it to allowed.list if needed",);
        }

        allowed
    }

    fn trim_port(host: &str) -> String {
        let mut tmp: Vec<_> = host.split(':').collect();
        if tmp.len() > 1 {
            tmp.pop(); // get rid of last element (port)
            tmp.join(":") //rejoin
        } else {
            host.to_string()
        }
    }

    /// Attempts to get the root domain, shorn of subdomains, using publicsuffix.
    /// If the domain is itself registered in publicsuffix (e.g. s3.amazonaws.com),
    /// then just use the full address as root.
    fn get_domain_root(&self, host: &str) -> Option<String> {
        match self.root_domain_list.parse_domain(host) {
            Ok(d) => Some(
                d.root()
                    .map(|root| root.to_string())
                    .unwrap_or_else(|| d.full().to_string()),
            ),
            Err(_) => {
                log::warn!("Error parsing domain: {:?}", host);
                None // domain couldn't be parsed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::{Deref, DerefMut};

    struct HostsStoreFixture {
        inner: HostsStore,
        // take ownership of temp file that will get cleaned up on drop (i.e. when test finishes)
        _tmp_file: tempfile::NamedTempFile,
    }

    impl Deref for HostsStoreFixture {
        type Target = HostsStore;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl DerefMut for HostsStoreFixture {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.inner
        }
    }

    impl HostsStoreFixture {
        fn new() -> HostsStoreFixture {
            let tmp_file = tempfile::NamedTempFile::new().unwrap();
            let inner = HostsStore::new(&tmp_file);
            HostsStoreFixture {
                inner,
                _tmp_file: tmp_file,
            }
        }
    }

    struct OutboundRequestFilterFixture {
        inner: OutboundRequestFilter,
        // take ownership of temp files that will get cleaned up on drop (i.e. when test finishes)
        _allow_tmp_file: tempfile::NamedTempFile,
        _unknown_tmp_file: tempfile::NamedTempFile,
    }

    impl Deref for OutboundRequestFilterFixture {
        type Target = OutboundRequestFilter;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl DerefMut for OutboundRequestFilterFixture {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.inner
        }
    }

    impl OutboundRequestFilterFixture {
        fn new() -> OutboundRequestFilterFixture {
            let allow_tmp_file = tempfile::NamedTempFile::new().unwrap();
            let unknown_tmp_file = tempfile::NamedTempFile::new().unwrap();
            let allowed = HostsStore::new(&allow_tmp_file);
            let unknown = HostsStore::new(&unknown_tmp_file);
            let standard = StandardList::new();

            let inner = OutboundRequestFilter::new(allowed.into(), standard, unknown);
            OutboundRequestFilterFixture {
                inner,
                _allow_tmp_file: allow_tmp_file,
                _unknown_tmp_file: unknown_tmp_file,
            }
        }
    }

    fn setup_empty() -> OutboundRequestFilterFixture {
        OutboundRequestFilterFixture::new()
    }

    fn setup_with_allowed(allowed: &[&str]) -> OutboundRequestFilterFixture {
        let allow_tmp_file = tempfile::NamedTempFile::new().unwrap();
        let unknown_tmp_file = tempfile::NamedTempFile::new().unwrap();
        let mut allowed_store = HostsStore::new(&allow_tmp_file);
        let unknown = HostsStore::new(&unknown_tmp_file);
        let standard = StandardList::new();

        for allow in allowed {
            allowed_store.add_host(allow)
        }

        let inner = OutboundRequestFilter::new(allowed_store.into(), standard, unknown);
        OutboundRequestFilterFixture {
            inner,
            _allow_tmp_file: allow_tmp_file,
            _unknown_tmp_file: unknown_tmp_file,
        }
    }

    #[cfg(test)]
    mod trimming_port_information {
        use super::*;

        #[test]
        fn happens_when_port_exists() {
            let host = "nymtech.net:9999";
            assert_eq!("nymtech.net", OutboundRequestFilter::trim_port(host));
        }

        #[test]
        fn doesnt_happen_when_no_port_exists() {
            let host = "nymtech.net";
            assert_eq!("nymtech.net", OutboundRequestFilter::trim_port(host));
        }
    }

    #[cfg(test)]
    mod getting_the_domain_root {
        use super::*;

        #[test]
        fn leaves_a_com_alone() {
            let filter = setup_empty();
            assert_eq!(
                Some("domain.com".to_string()),
                filter.get_domain_root("domain.com")
            )
        }

        #[test]
        fn trims_subdomains_from_com() {
            let filter = setup_empty();
            assert_eq!(
                Some("domain.com".to_string()),
                filter.get_domain_root("foomp.domain.com")
            )
        }

        #[test]
        fn works_for_non_com_roots() {
            let filter = setup_empty();
            assert_eq!(
                Some("domain.co.uk".to_string()),
                filter.get_domain_root("domain.co.uk")
            )
        }

        #[test]
        fn works_for_non_com_roots_with_subdomains() {
            let filter = setup_empty();
            assert_eq!(
                Some("domain.co.uk".to_string()),
                filter.get_domain_root("foomp.domain.co.uk")
            )
        }

        #[test]
        fn returns_none_on_garbage() {
            let filter = setup_empty();
            assert_eq!(None, filter.get_domain_root("::/&&%@"));
        }

        #[test]
        fn returns_full_on_suffix_domains() {
            let filter = setup_empty();
            assert_eq!(
                Some("s3.amazonaws.com".to_string()),
                filter.get_domain_root("s3.amazonaws.com")
            );
        }
    }

    #[cfg(test)]
    mod requests_to_unknown_hosts {
        use super::*;

        #[tokio::test]
        async fn are_not_allowed() {
            let host = "unknown.com";
            let filter = setup_empty();
            assert!(!filter.check(host).await);
        }

        #[tokio::test]
        async fn get_appended_once_to_the_unknown_hosts_list() {
            let host = "unknown.com";
            let filter = setup_empty();
            filter.check(host).await;
            assert_eq!(1, filter.unknown_hosts.lock().await.data.domains.len());
            assert!(filter
                .unknown_hosts
                .lock()
                .await
                .data
                .domains
                .contains("unknown.com"));
            filter.check(host).await;
            assert_eq!(1, filter.unknown_hosts.lock().await.data.domains.len());
            assert!(filter
                .unknown_hosts
                .lock()
                .await
                .data
                .domains
                .contains("unknown.com"));
        }
    }

    #[cfg(test)]
    mod requests_to_allowed_hosts {
        use super::*;

        #[tokio::test]
        async fn are_allowed() {
            let host = "nymtech.net";

            let filter = setup_with_allowed(&["nymtech.net"]);
            assert!(filter.check(host).await);
        }

        #[tokio::test]
        async fn are_allowed_for_subdomains() {
            let host = "foomp.nymtech.net";

            let filter = setup_with_allowed(&["nymtech.net"]);
            assert!(filter.check(host).await);
        }

        #[tokio::test]
        async fn are_not_appended_to_file() {
            let filter = setup_with_allowed(&["nymtech.net"]);

            // test initial state
            let lines =
                HostsStore::load_from_storefile(&filter.allowed_hosts.get().await.storefile)
                    .unwrap();
            assert_eq!(1, lines.len());

            assert!(filter.check("nymtech.net").await);

            // test state after we've checked to make sure no unexpected changes
            let lines =
                HostsStore::load_from_storefile(&filter.allowed_hosts.get().await.storefile)
                    .unwrap();
            assert_eq!(1, lines.len());
        }

        #[tokio::test]
        async fn are_allowed_for_ipv4_addresses() {
            let address_good = "1.1.1.1";
            let address_good_port = "1.1.1.1:1234";
            let address_bad = "1.1.1.2";

            let filter = setup_with_allowed(&["1.1.1.1"]);
            assert!(filter.check(address_good).await);
            assert!(filter.check(address_good_port).await);
            assert!(!filter.check(address_bad).await);
        }

        #[tokio::test]
        async fn are_allowed_for_ipv6_addresses() {
            let ip_v6_full = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
            let ip_v6_full_rendered = "2001:0db8:85a3::8a2e:0370:7334";
            let ip_v6_full_port = "[2001:0db8:85a3::8a2e:0370:7334]:1234";

            let ip_v6_semi = "2001:0db8::0001:0000";
            let ip_v6_semi_rendered = "2001:db8::1:0";

            let ip_v6_loopback_port = "[::1]:1234";

            let filter1 = setup_with_allowed(&[ip_v6_full, ip_v6_semi, "::1"]);
            let filter2 = setup_with_allowed(&[ip_v6_full_rendered, ip_v6_semi_rendered, "::1"]);

            assert!(filter1.check(ip_v6_full).await);
            assert!(filter1.check(ip_v6_full_rendered).await);
            assert!(filter1.check(ip_v6_full_port).await);
            assert!(filter1.check(ip_v6_semi).await);
            assert!(filter1.check(ip_v6_semi_rendered).await);
            assert!(filter1.check(ip_v6_loopback_port).await);

            assert!(filter2.check(ip_v6_full).await);
            assert!(filter2.check(ip_v6_full_rendered).await);
            assert!(filter2.check(ip_v6_full_port).await);
            assert!(filter2.check(ip_v6_semi).await);
            assert!(filter2.check(ip_v6_semi_rendered).await);
            assert!(filter2.check(ip_v6_loopback_port).await);
        }

        #[tokio::test]
        async fn are_allowed_for_ipv4_address_ranges() {
            let range1 = "127.0.0.1/32";
            let range2 = "1.2.3.4/24";

            let bottom_range2 = "1.2.3.0";
            let top_range2 = "1.2.3.255";

            let outside_range2 = "1.2.2.4";

            let filter = setup_with_allowed(&[range1, range2]);
            assert!(filter.check("127.0.0.1").await);
            assert!(filter.check("127.0.0.1:1234").await);
            assert!(filter.check(bottom_range2).await);
            assert!(filter.check(top_range2).await);
            assert!(!filter.check(outside_range2).await);
        }

        #[tokio::test]
        async fn are_allowed_for_ipv6_address_ranges() {
            let range = "2620:0:2d0:200::7/32";

            let bottom1 = "2620:0:0:0:0:0:0:0";
            let bottom2 = "2620::";

            let top = "2620:0:ffff:ffff:ffff:ffff:ffff:ffff";
            let mid = "2620:0:42::42";

            let filter = setup_with_allowed(&[range]);
            assert!(filter.check(bottom1).await);
            assert!(filter.check(bottom2).await);
            assert!(filter.check(top).await);
            assert!(filter.check(mid).await);
        }
    }

    #[cfg(test)]
    mod creating_a_new_host_store {
        use super::*;

        #[test]
        fn loads_its_host_list_from_storefile() {
            let mut host_store = HostsStoreFixture::new();

            host_store.add_host("nymtech.net");
            host_store.add_host("edwardsnowden.com");
            host_store.add_host("1.2.3.4");
            host_store.add_host("5.6.7.8/16");
            host_store.add_host("1:2:3::");
            host_store.add_host("5:6:7::/48");

            assert!(host_store.data.domains.contains("nymtech.net"));
            assert!(host_store.data.domains.contains("edwardsnowden.com"));

            assert!(host_store
                .data
                .ip_nets
                .contains(&"1.2.3.4".parse().unwrap()));
            assert!(host_store
                .data
                .ip_nets
                .contains(&"5.6.7.8/16".parse().unwrap()));
            assert!(host_store
                .data
                .ip_nets
                .contains(&"1:2:3::".parse().unwrap()));
            assert!(host_store
                .data
                .ip_nets
                .contains(&"5:6:7::/48".parse().unwrap()));
        }
    }
}
