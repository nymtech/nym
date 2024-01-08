// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::host::Host;
use crate::request_filter::allowed_hosts::group::HostsGroup;
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
        if storefile.exists() && !storefile.is_file() {
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
            log::error!("Couldn't write to file: {e}");
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
            log::trace!("Creating: {file:?}");
            File::create(file).unwrap();
        }
    }

    /// Loads the storefile contents into memory.
    pub(super) fn load_from_storefile<P>(filename: P) -> io::Result<Vec<Host>>
    where
        P: AsRef<Path>,
    {
        log::trace!("Loading from storefile: {}", filename.as_ref().display());
        let file = File::open(filename)?;
        let reader = BufReader::new(&file);
        let hosts = reader
            .lines()
            .filter_map(|line| {
                let line = line.expect("failed to read input file line!");
                trim_comment(&line)
            })
            .map(Host::from)
            .collect();
        Ok(hosts)
    }
}

fn trim_comment(line: &str) -> Option<String> {
    if let Some(content) = line.split('#').next() {
        let trim_content = content.trim().to_string();
        if trim_content.is_empty() {
            None
        } else {
            Some(trim_content)
        }
    } else {
        None
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

    #[test]
    fn trim_comments() {
        let entries = vec![
            "# keybase",
            "keybaseapi.com",
            "",
            "gist.githubusercontent.com",
            " ",
            "# healthcheck # foo",
            "nymtech.net",
            "# blockstream green wallet",
            "blockstream.info",
            "greenaddress.it",
            "91.108.56.0/22",
            "2001:b28:f23d::/48",
            "2001:67c:4e8::/48",
            "# nym matrix server",
            "# monero desktop - mainnet",
        ];
        let filtered_entries: Vec<_> = entries
            .iter()
            .filter_map(|line| trim_comment(line))
            .collect();
        assert_eq!(
            filtered_entries,
            vec![
                "keybaseapi.com",
                "gist.githubusercontent.com",
                "nymtech.net",
                "blockstream.info",
                "greenaddress.it",
                "91.108.56.0/22",
                "2001:b28:f23d::/48",
                "2001:67c:4e8::/48",
            ]
        );
    }
}
