// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, SocketAddr};

use super::HostsStore;

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
    pub(super) allowed_hosts: HostsStore,
    root_domain_list: publicsuffix::List,
    unknown_hosts: HostsStore,
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
        allowed_hosts: HostsStore,
        unknown_hosts: HostsStore,
    ) -> OutboundRequestFilter {
        let domain_list = match publicsuffix::List::fetch() {
            Ok(list) => list,
            Err(err) => panic!("Couldn't fetch domain list for request filtering, do you have an internet connection?: {err}"),
        };

        OutboundRequestFilter {
            allowed_hosts,
            root_domain_list: domain_list,
            unknown_hosts,
        }
    }

    /// Returns `true` if a host's root domain is in the `allowed_hosts` list.
    ///
    /// If it's not in the list, return `false` and write it to the `unknown_hosts` storefile.
    pub(crate) fn check(&mut self, host: &str) -> bool {
        // first check if it's a socket address (ip:port)
        // (this check is performed to not incorrectly strip what we think might be a port
        // from ipv6 address, as for example ::1 contains colons but has no port
        let allowed = if let Ok(socketaddr) = host.parse::<SocketAddr>() {
            if !self.allowed_hosts.contains_ip_address(socketaddr.ip()) {
                self.unknown_hosts.add_ip(socketaddr.ip());
                return false;
            }
            true
        } else if let Ok(ipaddr) = host.parse::<IpAddr>() {
            // then check if it was an ip address
            if !self.allowed_hosts.contains_ip_address(ipaddr) {
                self.unknown_hosts.add_ip(ipaddr);
                return false;
            }
            true
        } else {
            // finally, then assume it might be a domain
            let trimmed = Self::trim_port(host);
            if let Some(domain_root) = self.get_domain_root(&trimmed) {
                // it's a domain
                if !self.allowed_hosts.contains_domain(&domain_root) {
                    self.unknown_hosts.add_domain(&trimmed);
                    return false;
                }
                true
            } else {
                // it's something else, no idea what, probably some nonsense
                false
            }
        };

        if !allowed {
            log::warn!(
                "Blocked outbound connection to {:?}, add it to allowed.list if needed",
                &host
            );
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
    use std::{
        fs::{self, File},
        path::PathBuf,
    };

    use super::*;

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
        use std::path::PathBuf;

        use super::*;

        fn setup() -> OutboundRequestFilter {
            let base_dir = test_base_dir();
            let allowed_filename = PathBuf::from(format!("allowed-{}.list", random_string()));
            let unknown_filename = PathBuf::from(&format!("unknown-{}.list", random_string()));
            let allowed = HostsStore::new(base_dir.clone(), allowed_filename, None);
            let unknown = HostsStore::new(base_dir, unknown_filename, None);
            OutboundRequestFilter::new(allowed, unknown)
        }

        #[test]
        fn leaves_a_com_alone() {
            let filter = setup();
            assert_eq!(
                Some("domain.com".to_string()),
                filter.get_domain_root("domain.com")
            )
        }

        #[test]
        fn trims_subdomains_from_com() {
            let filter = setup();
            assert_eq!(
                Some("domain.com".to_string()),
                filter.get_domain_root("foomp.domain.com")
            )
        }

        #[test]
        fn works_for_non_com_roots() {
            let filter = setup();
            assert_eq!(
                Some("domain.co.uk".to_string()),
                filter.get_domain_root("domain.co.uk")
            )
        }

        #[test]
        fn works_for_non_com_roots_with_subdomains() {
            let filter = setup();
            assert_eq!(
                Some("domain.co.uk".to_string()),
                filter.get_domain_root("foomp.domain.co.uk")
            )
        }

        #[test]
        fn returns_none_on_garbage() {
            let filter = setup();
            assert_eq!(None, filter.get_domain_root("::/&&%@"));
        }

        #[test]
        fn returns_full_on_suffix_domains() {
            let filter = setup();
            assert_eq!(
                Some("s3.amazonaws.com".to_string()),
                filter.get_domain_root("s3.amazonaws.com")
            );
        }
    }

    #[cfg(test)]
    mod requests_to_unknown_hosts {
        use std::path::PathBuf;

        use super::*;

        fn setup() -> OutboundRequestFilter {
            let base_dir = test_base_dir();
            let allowed_filename = PathBuf::from(format!("allowed-{}.list", random_string()));
            let unknown_filename = PathBuf::from(&format!("unknown-{}.list", random_string()));
            let allowed = HostsStore::new(base_dir.clone(), allowed_filename, None);
            let unknown = HostsStore::new(base_dir, unknown_filename, None);
            OutboundRequestFilter::new(allowed, unknown)
        }

        #[test]
        fn are_not_allowed() {
            let host = "unknown.com";
            let mut filter = setup();
            assert!(!filter.check(host));
        }

        #[test]
        fn get_appended_once_to_the_unknown_hosts_list() {
            let host = "unknown.com";
            let mut filter = setup();
            filter.check(host);
            assert_eq!(1, filter.unknown_hosts.domains.len());
            assert!(filter.unknown_hosts.domains.contains("unknown.com"));
            filter.check(host);
            assert_eq!(1, filter.unknown_hosts.domains.len());
            assert!(filter.unknown_hosts.domains.contains("unknown.com"));
        }
    }

    #[cfg(test)]
    mod requests_to_allowed_hosts {
        use super::*;

        fn setup(allowed: &[&str]) -> OutboundRequestFilter {
            let (allowed_storefile, base_dir1, allowed_filename) = create_test_storefile();
            let (_, base_dir2, unknown_filename) = create_test_storefile();

            for allowed_host in allowed {
                HostsStore::append(&allowed_storefile, allowed_host)
            }

            let allowed = HostsStore::new(base_dir1, allowed_filename, None);
            let unknown = HostsStore::new(base_dir2, unknown_filename, None);
            OutboundRequestFilter::new(allowed, unknown)
        }

        #[test]
        fn are_allowed() {
            let host = "nymtech.net";

            let mut filter = setup(&["nymtech.net"]);
            assert!(filter.check(host));
        }

        #[test]
        fn are_allowed_for_subdomains() {
            let host = "foomp.nymtech.net";

            let mut filter = setup(&["nymtech.net"]);
            assert!(filter.check(host));
        }

        #[test]
        fn are_not_appended_to_file() {
            let mut filter = setup(&["nymtech.net"]);

            // test initial state
            let lines = HostsStore::load_from_storefile(&filter.allowed_hosts.storefile).unwrap();
            assert_eq!(1, lines.len());

            filter.check("nymtech.net");

            // test state after we've checked to make sure no unexpected changes
            let lines = HostsStore::load_from_storefile(&filter.allowed_hosts.storefile).unwrap();
            assert_eq!(1, lines.len());
        }

        #[test]
        fn are_allowed_for_ipv4_addresses() {
            let address_good = "1.1.1.1";
            let address_good_port = "1.1.1.1:1234";
            let address_bad = "1.1.1.2";

            let mut filter = setup(&["1.1.1.1"]);
            assert!(filter.check(address_good));
            assert!(filter.check(address_good_port));
            assert!(!filter.check(address_bad));
        }

        #[test]
        fn are_allowed_for_ipv6_addresses() {
            let ip_v6_full = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
            let ip_v6_full_rendered = "2001:0db8:85a3::8a2e:0370:7334";
            let ip_v6_full_port = "[2001:0db8:85a3::8a2e:0370:7334]:1234";

            let ip_v6_semi = "2001:0db8::0001:0000";
            let ip_v6_semi_rendered = "2001:db8::1:0";

            let ip_v6_loopback_port = "[::1]:1234";

            let mut filter1 = setup(&[ip_v6_full, ip_v6_semi, "::1"]);
            let mut filter2 = setup(&[ip_v6_full_rendered, ip_v6_semi_rendered, "::1"]);

            assert!(filter1.check(ip_v6_full));
            assert!(filter1.check(ip_v6_full_rendered));
            assert!(filter1.check(ip_v6_full_port));
            assert!(filter1.check(ip_v6_semi));
            assert!(filter1.check(ip_v6_semi_rendered));
            assert!(filter1.check(ip_v6_loopback_port));

            assert!(filter2.check(ip_v6_full));
            assert!(filter2.check(ip_v6_full_rendered));
            assert!(filter2.check(ip_v6_full_port));
            assert!(filter2.check(ip_v6_semi));
            assert!(filter2.check(ip_v6_semi_rendered));
            assert!(filter2.check(ip_v6_loopback_port));
        }

        #[test]
        fn are_allowed_for_ipv4_address_ranges() {
            let range1 = "127.0.0.1/32";
            let range2 = "1.2.3.4/24";

            let bottom_range2 = "1.2.3.0";
            let top_range2 = "1.2.3.255";

            let outside_range2 = "1.2.2.4";

            let mut filter = setup(&[range1, range2]);
            assert!(filter.check("127.0.0.1"));
            assert!(filter.check("127.0.0.1:1234"));
            assert!(filter.check(bottom_range2));
            assert!(filter.check(top_range2));
            assert!(!filter.check(outside_range2));
        }

        #[test]
        fn are_allowed_for_ipv6_address_ranges() {
            let range = "2620:0:2d0:200::7/32";

            let bottom1 = "2620:0:0:0:0:0:0:0";
            let bottom2 = "2620::";

            let top = "2620:0:ffff:ffff:ffff:ffff:ffff:ffff";
            let mid = "2620:0:42::42";

            let mut filter = setup(&[range]);
            assert!(filter.check(bottom1));
            assert!(filter.check(bottom2));
            assert!(filter.check(top));
            assert!(filter.check(mid));
        }
    }

    fn random_string() -> String {
        format!("{:?}", rand::random::<u32>())
    }

    fn test_base_dir() -> PathBuf {
        ["/tmp/nym-tests"].iter().collect()
    }

    fn create_test_storefile() -> (PathBuf, PathBuf, PathBuf) {
        let base_dir = test_base_dir();
        let filename = PathBuf::from(format!("hosts-store-{}.list", random_string()));
        let dirpath = base_dir.join("service-providers").join("network-requester");
        fs::create_dir_all(&dirpath)
            .unwrap_or_else(|_| panic!("could not create storage directory at {:?}", dirpath));
        let storefile = dirpath.join(&filename);
        File::create(&storefile).unwrap();
        (storefile, base_dir, filename)
    }

    #[cfg(test)]
    mod creating_a_new_host_store {
        use super::*;

        #[test]
        fn loads_its_host_list_from_storefile() {
            let (storefile, base_dir, filename) = create_test_storefile();
            HostsStore::append(&storefile, "nymtech.net");
            HostsStore::append(&storefile, "edwardsnowden.com");
            HostsStore::append(&storefile, "1.2.3.4");
            HostsStore::append(&storefile, "5.6.7.8/16");
            HostsStore::append(&storefile, "1:2:3::");
            HostsStore::append(&storefile, "5:6:7::/48");

            let host_store = HostsStore::new(base_dir, filename, None);
            assert!(host_store.domains.contains("nymtech.net"));
            assert!(host_store.domains.contains("edwardsnowden.com"));

            assert!(host_store.ip_nets.contains(&"1.2.3.4".parse().unwrap()));
            assert!(host_store.ip_nets.contains(&"5.6.7.8/16".parse().unwrap()));
            assert!(host_store.ip_nets.contains(&"1:2:3::".parse().unwrap()));
            assert!(host_store.ip_nets.contains(&"5:6:7::/48".parse().unwrap()));
        }
    }
}
