use std::fs::{self};
use std::path::PathBuf;

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

    pub(crate) fn is_unknown(&self, host: &str) -> bool {
        !self.allowed_hosts.contains(host)
    }
}

#[derive(Debug)]
struct HostsStore {
    storefile: String,
    hosts: Vec<String>,
}

impl HostsStore {
    fn new(filename: &str, hosts: Vec<String>) -> HostsStore {
        let dirpath = HostsStore::setup_storage_path();
        let filepath = dirpath.join(filename);
        HostsStore {
            storefile: filepath.to_str().unwrap().to_string(),
            hosts,
        }
    }

    fn contains(&self, host: &str) -> bool {
        self.hosts.contains(&host.to_string())
    }

    fn ensure_directory_exists(dirpath: &PathBuf) {
        fs::create_dir_all(dirpath);
    }

    fn maybe_add(&mut self, host: &str) {
        if !self.contains(&host) {
            self.hosts.push(host.to_string());
            self.append_to_file(host);
        }
    }

    fn append_to_file(&self, host: &str) -> std::io::Result<()> {
        fs::write(&self.storefile, host)?;
        Ok(())
    }
    fn setup_storage_path() -> PathBuf {
        let os_home_dir = dirs::home_dir().expect("no home directory known for this OS"); // grabs the OS default home dir
        let dirpath = os_home_dir
            .join(".nym")
            .join("service-providers")
            .join("socks5");
        HostsStore::ensure_directory_exists(&dirpath);
        dirpath
    }

    /// Reloads the allowed.list and unknown.list files into memory. Used primarily for testing.
    fn reload_from_disk(&self) {}
}
// Appender

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod requests_to_unknown_hosts {
        use super::*;

        fn setup() -> OutboundRequestFilter {
            let allowed = HostsStore::new(&format!("allowed-{}.list", random_string()), vec![]);
            let unknown = HostsStore::new(&format!("unknown-{}.list", random_string()), vec![]);
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

        #[test]
        fn are_written_once_to_file() {
            let host = "unknown.com";
            let mut filter = setup();
            filter.check(host);
            let lines = lines_from_file(&filter.unknown_hosts.storefile).unwrap();
            assert_eq!(1, lines.len());

            filter.check(host);
            let lines = lines_from_file(&filter.unknown_hosts.storefile).unwrap();
            assert_eq!(1, lines.len());
        }
    }
    #[cfg(test)]
    mod requests_to_allowed_hosts {
        use super::*;
        fn setup() -> OutboundRequestFilter {
            let allowed_hosts = HostsStore::new(
                &format!("allowed-{}.list", random_string()),
                vec!["nymtech.net".to_string()],
            );
            let unknown_hosts =
                HostsStore::new(&format!("unknown-{}.list", random_string()), vec![]);

            OutboundRequestFilter::new(allowed_hosts, unknown_hosts)
        }
        #[test]
        fn are_allowed() {
            let host = "nymtech.net";
            let mut filter = setup();
            assert_eq!(true, filter.check(host));
        }

        #[test]
        fn are_not_unknown() {
            let host = "nymtech.net";
            let mut filter = setup();
            assert_eq!(false, filter.is_unknown(host));
        }

        #[test]
        fn are_not_appended_to_file() {
            let host = "nymtech.net";
            let mut filter = setup();

            // test initial state
            let lines = lines_from_file(&filter.allowed_hosts.storefile).unwrap();
            assert_eq!(0, lines.len());

            filter.check(host);

            // test state after we've checked to make sure no unexpected changes
            let lines = lines_from_file(&filter.allowed_hosts.storefile).unwrap();
            assert_eq!(0, lines.len());
        }
    }

    fn random_string() -> String {
        format!("{:?}", rand::random::<u32>())
    }

    use io::BufReader;
    use std::fs::File;
    use std::io;
    use std::io::BufRead;
    use std::path::Path;

    fn lines_from_file<P>(filename: P) -> io::Result<Vec<String>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filename)?;
        let reader = BufReader::new(&file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
        Ok(lines)
    }
}
