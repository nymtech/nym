use fs::File;
use std::fs::{self};
use std::path::PathBuf;

/// Filters outbound requests based on what's in an `allowed_hosts` list.
///
/// Requests to unknown hosts are automatically written to an `unknown_hosts`
/// list so that they can be copy/pasted into the `allowed_hosts` list if desired.
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

/// A simple file-based store for information about allowed / unknown hosts.
#[derive(Debug)]
struct HostsStore {
    storefile: PathBuf,
    hosts: Vec<String>,
}

impl HostsStore {
    /// Constructs a new HostsStore
    fn new(base_dir: PathBuf, filename: PathBuf, hosts: Vec<String>) -> HostsStore {
        let storefile = HostsStore::setup_storefile(base_dir, filename);
        HostsStore { storefile, hosts }
    }

    /// Returns the default base directory for the storefile.
    ///
    /// This is split out so we can easily inject our own base_dir for unit tests.
    pub fn default_base_dir() -> PathBuf {
        dirs::home_dir()
            .expect("no home directory known for this OS")
            .join(".nym")
    }

    fn contains(&self, host: &str) -> bool {
        self.hosts.contains(&host.to_string())
    }

    fn ensure_storefile_exists(dirpath: &PathBuf) {}

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
    fn setup_storefile(base_dir: PathBuf, filename: PathBuf) -> PathBuf {
        let dirpath = base_dir.join("service-providers").join("socks5");
        fs::create_dir_all(&dirpath).expect(&format!(
            "could not create storage directory at {:?}",
            dirpath
        ));
        let storefile = dirpath.join(filename);
        let exists = std::path::Path::new(&storefile).exists();
        if !exists {
            File::create(&storefile).unwrap();
        }
        storefile
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
            let base_dir: PathBuf = ["/tmp/nym-tests"].iter().collect();
            let allowed_filename = PathBuf::from(format!("allowed-{}.list", random_string()));
            let unknown_filename = PathBuf::from(&format!("unknown-{}.list", random_string()));
            let allowed = HostsStore::new(base_dir.clone(), allowed_filename, vec![]);
            let unknown = HostsStore::new(base_dir.clone(), unknown_filename, vec![]);
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
            let base_dir: PathBuf = ["/tmp/nym"].iter().collect();
            let allowed_filename = PathBuf::from(format!("allowed-{}.list", random_string()));
            let unknown_filename = PathBuf::from(&format!("unknown-{}.list", random_string()));
            let allowed = HostsStore::new(
                base_dir.clone(),
                allowed_filename,
                vec!["nymtech.net".to_string()],
            );
            let unknown = HostsStore::new(base_dir.clone(), unknown_filename, vec![]);

            OutboundRequestFilter::new(allowed, unknown)
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
