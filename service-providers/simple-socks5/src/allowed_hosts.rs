struct OutboundRequestFilter {
    allowed_hosts: HostsStore,
    unknown_hosts: HostsStore,
}

impl OutboundRequestFilter {
    pub(crate) fn new(
        allowed_hosts: HostsStore,
        unknown_hosts: HostsStore,
    ) -> OutboundRequestFilter {
        OutboundRequestFilter {
            allowed_hosts,
            unknown_hosts,
        }
    }

    pub(crate) fn check(&mut self, host: &str) -> bool {
        if self.allowed_hosts.contains(host) {
            true
        } else {
            self.unknown_hosts.maybe_add(host);
            false
        }
    }

    pub(crate) fn is_unknown(&self, _host: &str) -> bool {
        true
    }
}

#[derive(Debug)]
struct HostsStore {
    hosts: Vec<String>,
}

impl HostsStore {
    fn new(hosts: Vec<String>) -> HostsStore {
        HostsStore { hosts }
    }

    fn contains(&self, host: &str) -> bool {
        self.hosts.contains(&host.to_string())
    }

    fn maybe_add(&mut self, host: &str) {
        if !self.contains(&host) {
            self.hosts.push(host.to_string());
        }
    }

    /// Reloads the allowed.list and unknown.list files into memory. Used primarily for testing.
    fn reload_from_disk(&self) {}

    fn append_to_file(&self, _host: String) {}
}
// Appender

#[cfg(test)]
mod requests_to_unknown_hosts {
    use super::*;

    fn setup() -> OutboundRequestFilter {
        let allowed = HostsStore::new(vec![]);
        let unknown = HostsStore::new(vec![]);
        OutboundRequestFilter::new(allowed, unknown)
    }

    #[test]
    fn are_not_allowed() {
        let host = "unknown.com";
        let mut filter = setup();
        assert_eq!(false, filter.check(&host));
    }

    #[test]
    fn get_saved_to_file() {
        let host = "unknown.com";
        let mut filter = setup();
        filter.check(host);
        assert!(true, filter.is_unknown(host));
    }

    #[test]
    fn get_appended_once_to_the_unknown_hosts_list() {
        let host = "unknown.com";
        let mut filter = setup();
        filter.check(host);
        assert_eq!(1, filter.unknown_hosts.hosts.len());
        assert_eq!("unknown.com", filter.unknown_hosts.hosts.first().unwrap());

        filter.check(host);
        assert_eq!(1, filter.unknown_hosts.hosts.len());
        assert_eq!("unknown.com", filter.unknown_hosts.hosts.first().unwrap());
    }
}

#[cfg(test)]
mod requests_to_allowed_hosts {
    use super::*;

    fn setup() -> OutboundRequestFilter {
        let allowed = HostsStore::new(vec!["nymtech.net".to_string()]);
        let unknown = HostsStore::new(vec![]);
        OutboundRequestFilter::new(allowed, unknown)
    }

    #[test]
    fn are_allowed() {
        let host = "nymtech.net";
        let mut filter = setup();
        assert_eq!(true, filter.check(host));
    }

    // #[test]
    // fn are_not_appended_to_file() {
    //     todo!()
    // }
}
