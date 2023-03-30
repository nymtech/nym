use ipnetwork::IpNetwork;

// used for parsing file content
#[derive(Debug)]
pub(crate) enum Host {
    Domain(String),
    IpNetwork(IpNetwork),
}

// TODO: perhaps in the future it should do some domain validation?
//
// So for example if somebody put some nonsense in the whitelist file like "foomp",
// it would get rejected?
impl From<String> for Host {
    fn from(raw: String) -> Self {
        if let Ok(ipnet) = raw.parse() {
            Host::IpNetwork(ipnet)
        } else {
            Host::Domain(raw)
        }
    }
}
