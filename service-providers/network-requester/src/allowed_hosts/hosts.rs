use super::host::Host;
use crate::allowed_hosts::group::HostsGroup;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader},
    net::IpAddr,
    path::{Path, PathBuf},
};

/// A simple file-backed store for information about allowed / unknown hosts.
/// It ignores any port information.
#[derive(Debug)]
pub(crate) struct HostsStore {
    pub(super) storefile: PathBuf,
    pub(super) data: HostsGroup,
}

impl HostsStore {
    /// Constructs a new HostsStore. If the storefile does not exist, it will be created.
    ///
    /// You can inject a list of standard hosts that you want to support, in addition to the ones
    /// in the user-defined storefile.
    pub(crate) fn new(base_dir: PathBuf, filename: PathBuf) -> HostsStore {
        let storefile = Self::setup_storefile(base_dir, filename);
        let hosts = Self::load_from_storefile(&storefile)
            .unwrap_or_else(|_| panic!("Could not load hosts from storefile at {storefile:?}"));

        HostsStore {
            storefile,
            data: HostsGroup::new(hosts),
        }
    }

    pub(crate) fn try_reload(&mut self) -> io::Result<()> {
        let hosts = Self::load_from_storefile(&self.storefile)?;
        self.data = HostsGroup::new(hosts);
        Ok(())
    }

    pub(crate) fn contains_domain(&self, host: &str) -> bool {
        self.data.contains_domain(host)
    }

    pub(super) fn contains_ip_address(&self, address: IpAddr) -> bool {
        self.data.contains_ip_address(address)
    }

    pub(super) fn add_ip(&mut self, ip: IpAddr) {
        if !self.contains_ip_address(ip) {
            self.data.add_ip(ip);
            self.append_to_file(&ip.to_string());
        }
    }

    pub(super) fn add_domain(&mut self, domain: &str) {
        if !self.contains_domain(domain) {
            self.data.add_domain(domain);
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

        if let Err(e) = writeln!(file, "{text}") {
            eprintln!("Couldn't write to file: {e}");
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
            .unwrap_or_else(|_| panic!("could not create storage directory at {dirpath:?}"));
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
        let store = HostsStore::new(PathBuf::from("/tmp"), PathBuf::from("foomp.db"));
        assert_eq!(store.data.domains.len(), 0);
    }
}
