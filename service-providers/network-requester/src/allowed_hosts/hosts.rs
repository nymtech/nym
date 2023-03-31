use super::host::Host;
use crate::allowed_hosts::group::HostsGroup;
use ipnetwork::IpNetwork;
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
    pub(crate) fn new<P: AsRef<Path>>(storefile: P) -> HostsStore {
        let storefile = storefile.as_ref().to_path_buf();
        if !storefile.is_file() {
            // there's no error handling in here and I'm not going to be changing it right now.
            panic!(
                "the provided storefile {:?} is not a valid file!",
                storefile
            )
        }

        Self::setup_storefile(&storefile);
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

    #[allow(unused)]
    pub(super) fn contains_ipnetwork(&self, network: IpNetwork) -> bool {
        self.data.contains_ip_network(network)
    }

    #[allow(unused)]
    pub(super) fn add_host<H: Into<Host>>(&mut self, host: H) {
        match host.into() {
            Host::Domain(domain) => self.add_domain(&domain),
            Host::IpNetwork(ipnet) => self.add_ipnet(ipnet),
        }
    }

    #[allow(unused)]
    pub(super) fn add_ipnet(&mut self, network: IpNetwork) {
        if !self.contains_ipnetwork(network) {
            self.data.add_ipnet(network);
            self.append_to_file(&network.to_string());
        }
    }

    pub(super) fn add_ip(&mut self, ip: IpAddr) {
        if !self.contains_ip_address(ip) {
            self.data.add_ipnet(ip);
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

    fn setup_storefile(file: &PathBuf) {
        if !file.exists() {
            let parent_dir = file
                .parent()
                .expect("parent dir does not exist for {file:?}");
            fs::create_dir_all(parent_dir)
                .unwrap_or_else(|_| panic!("could not create storage directory at {parent_dir:?}"));
            File::create(file).unwrap();
        }
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
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let store = HostsStore::new(temp_file);
        assert_eq!(store.data.domains.len(), 0);
    }
}
