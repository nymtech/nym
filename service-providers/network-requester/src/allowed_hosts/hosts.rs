use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader},
    net::IpAddr,
    path::{Path, PathBuf},
};

use super::host::Host;
use ipnetwork::IpNetwork;

/// A simple file-backed store for information about allowed / unknown hosts.
/// It ignores any port information.
#[derive(Debug)]
pub(crate) struct HostsStore {
    pub(super) storefile: PathBuf,

    pub(super) domains: HashSet<String>,
    pub(super) ip_nets: Vec<IpNetwork>,
}

impl HostsStore {
    /// Constructs a new HostsStore. If the storefile does not exist, it will be created.
    ///
    /// You can inject a list of standard hosts that you want to support, in addition to the ones
    /// in the user-defined storefile.
    pub(crate) fn new(
        base_dir: PathBuf,
        filename: PathBuf,
        standard_hosts: Option<Vec<Host>>,
    ) -> HostsStore {
        let storefile = Self::setup_storefile(base_dir, filename);
        let mut hosts = Self::load_from_storefile(&storefile)
            .unwrap_or_else(|_| panic!("Could not load hosts from storefile at {:?}", storefile));

        if standard_hosts.is_some() {
            hosts.extend(standard_hosts.unwrap_or_default());
        }

        let (domains, ip_nets): (Vec<_>, Vec<_>) =
            hosts.into_iter().partition(|host| host.is_domain());

        HostsStore {
            storefile,
            domains: domains.into_iter().map(Host::extract_domain).collect(),
            ip_nets: ip_nets.into_iter().map(Host::extract_ipnetwork).collect(),
        }
    }

    pub(crate) fn contains_domain(&self, host: &str) -> bool {
        self.domains.contains(&host.to_string())
    }

    pub(super) fn contains_ip_address(&self, address: IpAddr) -> bool {
        for ip_net in &self.ip_nets {
            if ip_net.contains(address) {
                return true;
            }
        }

        false
    }

    pub(super) fn add_ip(&mut self, ip: IpAddr) {
        if !self.contains_ip_address(ip) {
            self.ip_nets.push(ip.into());
            self.append_to_file(&ip.to_string());
        }
    }

    pub(super) fn add_domain(&mut self, domain: &str) {
        if !self.contains_domain(domain) {
            self.domains.insert(domain.to_string());
            self.append_to_file(domain);
        }
    }

    /// Appends a line of `text` to the storefile at `path`
    pub(super) fn append(path: &Path, text: &str) {
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

    /// Returns the default base directory for the storefile.
    ///
    /// This is split out so we can easily inject our own base_dir for unit tests.
    pub fn default_base_dir() -> PathBuf {
        dirs::home_dir()
            .expect("no home directory known for this OS")
            .join(".nym")
    }

    fn setup_storefile(base_dir: PathBuf, filename: PathBuf) -> PathBuf {
        let dirpath = base_dir.join("service-providers").join("network-requester");
        fs::create_dir_all(&dirpath)
            .unwrap_or_else(|_| panic!("could not create storage directory at {:?}", dirpath));
        let storefile = dirpath.join(filename);
        let exists = std::path::Path::new(&storefile).exists();
        if !exists {
            File::create(&storefile).unwrap();
        }
        storefile
    }

    /// Loads the storefile contents into memory.
    pub(super) fn load_from_storefile<P>(filename: P) -> io::Result<Vec<Host>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filename)?;
        let reader = BufReader::new(&file);
        Ok(reader
            .lines()
            .map(|line| Host::from(line.expect("failed to read input file line!")))
            .collect())
    }
}

#[cfg(test)]
mod constructor_tests {
    use super::*;

    #[test]
    fn works_with_no_standard_hosts() {
        let store = HostsStore::new(PathBuf::from("/tmp"), PathBuf::from("foomp.db"), None);
        assert_eq!(store.domains.len(), 0);
    }

    #[test]
    fn includes_standard_hosts_when_they_are_supplied() {
        let store = HostsStore::new(
            PathBuf::from("/tmp"),
            PathBuf::from("foomp.db"),
            Some(vec![
                Host::from(String::from("google.com")),
                Host::from(String::from("foomp.com")),
            ]),
        );
        assert_eq!(store.domains.len(), 2);
        assert!(store.contains_domain("google.com"));
        assert!(store.contains_domain("foomp.com"));
    }
}
