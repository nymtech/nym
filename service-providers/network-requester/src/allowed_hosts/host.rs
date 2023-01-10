use ipnetwork::IpNetwork;

// used for parsing file content
pub(crate) enum Host {
    Domain(String),
    IpNetwork(IpNetwork),
}

// TODO: perphaps in the future it should do some domain validation?
// so for example if somebody put some nonsense in the whitelist file like "foomp", it would get
// rejected?
impl From<String> for Host {
    fn from(raw: String) -> Self {
        if let Ok(ipnet) = raw.parse() {
            Host::IpNetwork(ipnet)
        } else {
            Host::Domain(raw)
        }
    }
}

impl Host {
    pub(crate) fn is_domain(&self) -> bool {
        matches!(self, Host::Domain(..))
    }

    pub(crate) fn extract_domain(self) -> String {
        match self {
            Host::Domain(domain) => domain,
            _ => panic!("called extract domain on an ipnet!"),
        }
    }

    pub(crate) fn extract_ipnetwork(self) -> IpNetwork {
        match self {
            Host::IpNetwork(ipnet) => ipnet,
            _ => panic!("called extract ipnet on a domain!"),
        }
    }
}
