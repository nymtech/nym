use fs::OpenOptions;
use io::BufReader;
use publicsuffix::{errors, List};
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;

/// Filters outbound requests based on what's in an `allowed_hosts` list.
///
/// Requests to unknown hosts are automatically written to an `unknown_hosts`
/// list so that they can be copy/pasted into the `allowed_hosts` list if desired.
/// This may be handy for service provider node operators who want to be able to look in the
/// `unknown_hosts` file and allow new hosts (e.g. if a wallet has added a new outbound request
/// which needs to be allowed).
pub(crate) struct OutboundRequestFilter {
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

    /// Returns `true` if a host's domain is in the `allowed_hosts` list.
    ///
    /// If it's not in the list, return `false` and write it to the `unknown_hosts` storefile.
    pub(crate) fn check(&mut self, host: &str) -> bool {
        let trimmed = Self::trim_port(host);
        let domain_root = Self::get_domain_root(&trimmed).unwrap();
        if self.allowed_hosts.contains(&domain_root) {
            true
        } else {
            self.unknown_hosts.maybe_add(&domain_root);
            false
        }
    }

    fn trim_port(host: &str) -> String {
        let mut tmp: Vec<&str> = host.split(":").collect();
        if tmp.len() > 1 {
            tmp.pop(); // get rid of last element (port)
            let out = tmp.join(":");
            out
        } else {
            host.to_string()
        }
    }

    /// Attempts to get the root domain, shorn of port, subdomains, etc.
    fn get_domain_root(host: &str) -> Result<String, errors::ErrorKind> {
        let list = List::fetch()?;
        let domain = list.parse_domain(host)?;
        let root = domain.root().unwrap();
        Ok(root.to_string())
    }
}

/// A simple file-based store for information about allowed / unknown hosts.
#[derive(Debug)]
pub(crate) struct HostsStore {
    storefile: PathBuf,
    hosts: Vec<String>,
}

impl HostsStore {
    /// Constructs a new HostsStore
    pub(crate) fn new(base_dir: PathBuf, filename: PathBuf) -> HostsStore {
        let storefile = HostsStore::setup_storefile(base_dir, filename);
        let hosts = HostsStore::load_from_storefile(&storefile).expect(&format!(
            "Could not load hosts from storefile at {:?}",
            storefile
        ));
        HostsStore { storefile, hosts }
    }

    fn append(path: &PathBuf, text: &str) {
        use std::io::Write;
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .unwrap();

        if let Err(e) = writeln!(file, "{}", text) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }

    fn append_to_file(&self, host: &str) {
        HostsStore::append(&self.storefile, host);
    }

    fn contains(&self, host: &str) -> bool {
        self.hosts.contains(&host.to_string())
    }

    /// Returns the default base directory for the storefile.
    ///
    /// This is split out so we can easily inject our own base_dir for unit tests.
    pub fn default_base_dir() -> PathBuf {
        dirs::home_dir()
            .expect("no home directory known for this OS")
            .join(".nym")
    }

    fn maybe_add(&mut self, host: &str) {
        if !self.contains(host) {
            self.hosts.push(host.to_string());
            self.append_to_file(host);
        }
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

    /// Loads the storefile contents into memory.
    fn load_from_storefile<P>(filename: P) -> io::Result<Vec<String>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filename)?;
        let reader = BufReader::new(&file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
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
    mod getting_the_root_domain {
        use super::*;

        #[test]
        fn gets_a_com_tld_ok() {
            let host = "domain.com";
            assert_eq!(
                "domain.com",
                OutboundRequestFilter::get_domain_root(host).unwrap()
            )
        }

        #[test]
        fn trims_subdomains() {
            let host = "foomp.domain.com";
            assert_eq!(
                "domain.com",
                OutboundRequestFilter::get_domain_root(host).unwrap()
            )
        }

        #[test]
        fn works_for_non_com_roots() {
            let host = "domain.co.uk";
            assert_eq!(
                "domain.co.uk",
                OutboundRequestFilter::get_domain_root(host).unwrap()
            )
        }

        #[test]
        fn works_for_non_com_roots_with_subdomains() {
            let host = "foomp.domain.co.uk";
            assert_eq!(
                "domain.co.uk",
                OutboundRequestFilter::get_domain_root(host).unwrap()
            )
        }
    }

    #[cfg(test)]
    mod requests_to_unknown_hosts {
        use super::*;

        fn setup() -> OutboundRequestFilter {
            let base_dir = test_base_dir();
            let allowed_filename = PathBuf::from(format!("allowed-{}.list", random_string()));
            let unknown_filename = PathBuf::from(&format!("unknown-{}.list", random_string()));
            let allowed = HostsStore::new(base_dir.clone(), allowed_filename);
            let unknown = HostsStore::new(base_dir.clone(), unknown_filename);
            OutboundRequestFilter::new(allowed, unknown)
        }

        #[test]
        fn are_not_allowed() {
            let host = "unknown.com";
            let mut filter = setup();
            assert_eq!(false, filter.check(&host));
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
            let (allowed_storefile, base_dir1, allowed_filename) = create_test_storefile();
            let (_, base_dir2, unknown_filename) = create_test_storefile();
            HostsStore::append(&allowed_storefile, "nymtech.net");

            let allowed = HostsStore::new(base_dir1, allowed_filename.to_path_buf());
            let unknown = HostsStore::new(base_dir2, unknown_filename.to_path_buf());
            OutboundRequestFilter::new(allowed, unknown)
        }
        #[test]
        fn are_allowed() {
            let host = "nymtech.net";

            let mut filter = setup();
            assert_eq!(true, filter.check(host));
        }

        #[test]
        fn are_allowed_for_subdomains() {
            let host = "foomp.nymtech.net";

            let mut filter = setup();
            assert_eq!(true, filter.check(host));
        }

        #[test]
        fn are_not_appended_to_file() {
            let mut filter = setup();

            // test initial state
            let lines = HostsStore::load_from_storefile(&filter.allowed_hosts.storefile).unwrap();
            assert_eq!(1, lines.len());

            filter.check("nymtech.net");

            // test state after we've checked to make sure no unexpected changes
            let lines = HostsStore::load_from_storefile(&filter.allowed_hosts.storefile).unwrap();
            assert_eq!(1, lines.len());
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
        let dirpath = base_dir.join("service-providers").join("socks5");
        fs::create_dir_all(&dirpath).expect(&format!(
            "could not create storage directory at {:?}",
            dirpath
        ));
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

            let host_store = HostsStore::new(base_dir, filename);
            assert_eq!(vec!["nymtech.net", "edwardsnowden.com"], host_store.hosts);
        }
    }
}
