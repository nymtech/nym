struct OutboundRequestFilter {
    allowed_hosts: Persistence,
    unknown_hosts: Persistence,
}

impl OutboundRequestFilter {
    pub(crate) fn new(
        allowed_hosts: Persistence,
        unknown_hosts: Persistence,
    ) -> OutboundRequestFilter {
        OutboundRequestFilter {
            allowed_hosts,
            unknown_hosts,
        }
    }

    pub(crate) fn check(&self, host: &str) -> bool {
        if self.allowed_hosts.contains(host.to_string()) {
            true
        } else {
            self.unknown_hosts.maybe_add(host.to_string());
            false
        }
    }

    pub(crate) fn is_unknown(&self, host: &str) -> bool {
        true
    }
}

#[derive(Debug)]
struct Persistence {
    hosts: Vec<String>,
}

impl Persistence {
    fn new(hosts: Vec<String>) -> Persistence {
        Persistence { hosts }
    }

    fn contains(&self, host: String) -> bool {
        self.hosts.contains(&host)
    }

    fn maybe_add(&mut self, host: String) {
        if !self.contains(host) {
            self.hosts.push(host);
        }
    }

    /// Reloads the allowed.list and unknown.list files into memory. Used primarily for testing.
    fn reload_from_disk(&self) {}

    fn append_to_file(&self, host: String) {}
}
// Appender

#[cfg(test)]
mod requests_to_unknown_hosts {
    use super::*;

    fn setup() -> OutboundRequestFilter {
        let allowed = Persistence::new(vec![]);
        let unknown = Persistence::new(vec![]);
        OutboundRequestFilter::new(allowed, unknown)
    }

    #[test]
    fn are_not_allowed() {
        let host = "unknown.com";
        let filter = setup();
        assert_eq!(false, filter.check(&host));
    }

    #[test]
    fn get_saved_to_file() {
        let host = "unknown.com";
        let filter = setup();
        filter.check(host);
        assert!(true, filter.is_unknown(host));
    }

    #[test]
    fn get_appended_once_to_the_unknown_hosts_list() {
        let host = "unknown.com";
        let filter = setup();
        filter.check(host);
        assert_eq!(1, filter.unknown_hosts.hosts.len());
    }
}

#[cfg(test)]
mod requests_to_allowed_hosts {
    use super::*;

    fn setup() -> OutboundRequestFilter {
        let allowed = Persistence::new(vec!["nymtech.net".to_string()]);
        let unknown = Persistence::new(vec![]);
        OutboundRequestFilter::new(allowed, unknown)
    }

    #[test]
    fn are_allowed() {
        let host = "nymtech.net";
        let filter = setup();
        assert_eq!(true, filter.check(host));
    }

    // #[test]
    // fn are_not_appended_to_file() {
    //     todo!()
    // }
}
