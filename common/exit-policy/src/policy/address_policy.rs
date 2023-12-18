// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Implements address policies, based on a series of accept/reject
//! rules.

use crate::policy::error::PolicyError;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::str::FromStr;
use tracing::trace;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum AddressPolicyAction {
    /// A rule that accepts matching address:port combinations on IPv4 and IPv6.
    Accept,

    /// A rule that rejects matching address:port combinations on IPv4 and IPv6.
    Reject,

    /// A rule that accepts matching address:port combinations on IPv6 only.
    Accept6,

    /// A rule that rejects matching address:port combinations on IPv6 only.
    Reject6,
}

impl AddressPolicyAction {
    pub fn is_accept(&self) -> bool {
        matches!(
            self,
            AddressPolicyAction::Accept | AddressPolicyAction::Accept6
        )
    }
}

impl FromStr for AddressPolicyAction {
    type Err = PolicyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "accept" => Ok(AddressPolicyAction::Accept),
            "reject" => Ok(AddressPolicyAction::Reject),
            "accept6" => Ok(AddressPolicyAction::Accept6),
            "reject6" => Ok(AddressPolicyAction::Reject6),
            other => Err(PolicyError::InvalidPolicyAction {
                action: other.to_string(),
            }),
        }
    }
}

impl Display for AddressPolicyAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AddressPolicyAction::Accept => write!(f, "accept"),
            AddressPolicyAction::Reject => write!(f, "reject"),
            AddressPolicyAction::Accept6 => write!(f, "accept6"),
            AddressPolicyAction::Reject6 => write!(f, "reject6"),
        }
    }
}

/// A sequence of rules that are applied to an address:port until one
/// matches.
///
/// Each rule is of the form "accept(6) PATTERN" or "reject(6) PATTERN",
/// where every pattern describes a set of addresses and ports.
/// Address sets are given as a prefix of 0-128 bits that the address
/// must have; port sets are given as a low-bound and high-bound that
/// the target port might lie between.
///
/// An example IPv4 policy might be:
///
/// ```text
///  reject *:25
///  reject 127.0.0.0/8:*
///  reject 192.168.0.0/16:*
///  accept *:80
///  accept *:443
///  accept *:9000-65535
///  reject *:*
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", aliases(ExitPolicy))]
pub struct AddressPolicy {
    /// A list of rules to apply to find out whether an address is
    /// contained by this policy.
    ///
    /// The rules apply in order; the first one to match determines
    /// whether the address is accepted or rejected.
    pub(crate) rules: Vec<AddressPolicyRule>,
}

impl AddressPolicy {
    /// Create a new AddressPolicy that matches nothing.
    pub const fn new() -> Self {
        AddressPolicy { rules: Vec::new() }
    }

    /// Create a new AddressPolicy that matches everything.
    pub fn new_open() -> Self {
        AddressPolicy {
            rules: vec![AddressPolicyRule::new(
                AddressPolicyAction::Accept,
                AddressPortPattern {
                    ip_pattern: IpPattern::Star,
                    ports: PortRange::new_all(),
                },
            )],
        }
    }

    /// Check whether this AddressPolicy matches all patterns.
    pub fn is_open(&self) -> bool {
        if self.rules.len() != 1 {
            return false;
        }

        let rule = &self.rules[0];

        rule.action == AddressPolicyAction::Accept
            && rule.pattern.ip_pattern == IpPattern::Star
            && rule.pattern.ports.is_all()
    }

    /// Attempts to parse the AddressPolicy out of raw torrc representation.
    pub fn parse_from_torrc<S: AsRef<str>>(raw: S) -> Result<Self, PolicyError> {
        crate::parse_exit_policy(raw)
    }

    /// Formats the AddressPolicy with torrc representation
    pub fn format_as_torrc(&self) -> String {
        crate::format_exit_policy(self)
    }

    /// Apply this policy to an address:port combination
    ///
    /// We do this by applying each rule in sequence, until one
    /// matches.
    ///
    /// Returns None if no rule matches.
    pub fn allows(&self, addr: &IpAddr, port: u16) -> Option<bool> {
        self.rules
            .iter()
            .find(|rule| rule.pattern.matches(addr, port))
            .map(|rule| {
                trace!("'{addr}:{port}' is covered by rule '{rule}'");
                rule.action.is_accept()
            })
    }

    /// As allows, but accept a SocketAddr.
    pub fn allows_sockaddr(&self, addr: &SocketAddr) -> Option<bool> {
        self.allows(&addr.ip(), addr.port())
    }

    /// Add a new rule to this policy.
    ///
    /// The newly added rule is applied _after_ all previous rules.
    /// It matches all addresses and ports covered by AddressPortPattern.
    ///
    /// If accept is true, the rule is to accept addresses that match;
    /// if accept is false, the rule rejects such addresses.
    pub fn push(&mut self, action: AddressPolicyAction, pattern: AddressPortPattern) {
        self.rules.push(AddressPolicyRule { action, pattern })
    }

    /// As push, but accepts a AddressPolicyRule.
    pub fn push_rule(&mut self, rule: AddressPolicyRule) {
        self.rules.push(rule)
    }
}

/// A single rule in an address policy.
///
/// Contains a pattern, what to do with things that match it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AddressPolicyRule {
    /// What do we do with items that match the pattern?
    action: AddressPolicyAction,

    /// What pattern are we trying to match?
    pattern: AddressPortPattern,
}

impl FromStr for AddressPolicyRule {
    type Err = PolicyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // split on the first space, i.e. separation between the action and the pattern
        let Some((action, pattern)) = s.split_once(' ') else {
            return Err(PolicyError::MalformedAddressPolicy { raw: s.to_string() });
        };

        Ok(AddressPolicyRule {
            action: action.parse()?,
            pattern: pattern.parse()?,
        })
    }
}

impl AddressPolicyRule {
    pub fn new(action: AddressPolicyAction, pattern: AddressPortPattern) -> Self {
        AddressPolicyRule { action, pattern }
    }
}
impl Display for AddressPolicyRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.action, self.pattern)
    }
}

/// A pattern that may or may not match an address and port.
///
/// Each AddrPortPattern has an IP pattern, which matches a set of
/// addresses by prefix, and a port pattern, which matches a range of
/// ports.
///
/// # Example
///
/// ```
/// use nym_exit_policy::policy::AddressPortPattern;
/// use std::net::{IpAddr,Ipv4Addr};
/// let localhost = IpAddr::V4(Ipv4Addr::new(127,3,4,5));
/// let not_localhost = IpAddr::V4(Ipv4Addr::new(192,0,2,16));
/// let pat: AddressPortPattern = "127.0.0.0/8:*".parse().unwrap();
///
/// assert!(pat.matches(&localhost, 22));
/// assert!(!pat.matches(&not_localhost, 22));
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AddressPortPattern {
    /// A pattern to match somewhere between zero and all IP addresses.
    #[serde(with = "stringified_ip_pattern")]
    #[cfg_attr(feature = "openapi", schema(example = "1.2.3.6/16", value_type = String))]
    pub(crate) ip_pattern: IpPattern,

    /// A pattern to match a range of ports.
    pub(crate) ports: PortRange,
}

mod stringified_ip_pattern {
    use super::IpPattern;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S: Serializer>(pattern: &IpPattern, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&pattern.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<IpPattern, D::Error> {
        let s = <String>::deserialize(deserializer)?;
        IpPattern::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl AddressPortPattern {
    /// Return true iff this pattern matches a given address and port.
    pub fn matches(&self, addr: &IpAddr, port: u16) -> bool {
        // For backward compatibility, we treat port 0 as a wildcard until all gateways have
        // upgraded, at which point we can add *:0 to the policy list.
        if port == 0 {
            self.ip_pattern.matches(addr)
        } else {
            self.ip_pattern.matches(addr) && self.ports.contains(port)
        }
    }

    /// As matches, but accept a SocketAddr.
    pub fn matches_sockaddr(&self, addr: &SocketAddr) -> bool {
        self.matches(&addr.ip(), addr.port())
    }
}

impl Display for AddressPortPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ip_pattern, self.ports)
    }
}

impl FromStr for AddressPortPattern {
    type Err = PolicyError;

    fn from_str(s: &str) -> Result<Self, PolicyError> {
        let last_colon = s
            .rfind(':')
            .ok_or(PolicyError::MalformedAddressPortPattern { raw: s.to_string() })?;

        // doesn't have enough chars to cover the port, even if its a wildcard
        if s.len() < last_colon + 2 {
            return Err(PolicyError::MalformedAddressPortPattern { raw: s.to_string() });
        }

        let ip_pattern = s[..last_colon].parse()?;
        let ports = s[last_colon + 1..].parse()?;

        Ok(AddressPortPattern { ip_pattern, ports })
    }
}

/// A pattern that matches one or more IP addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IpPattern {
    /// Match all addresses.
    Star,

    /// Match all IPv4 addresses.
    V4Star,

    /// Match all IPv6 addresses.
    V6Star,

    /// Match all IPv4 addresses beginning with a given prefix and mask.
    V4 { addr_prefix: Ipv4Addr, mask: u8 },

    /// Match all IPv6 addresses beginning with a given prefix and mask.
    V6 { addr_prefix: Ipv6Addr, mask: u8 },
}

impl IpPattern {
    /// Construct an IpPattern that matches the first `mask` bits of `addr`.
    fn from_addr_and_mask(address: IpAddr, target_mask: u8) -> Result<Self, PolicyError> {
        match (address, target_mask) {
            (IpAddr::V4(_), 0) => Ok(IpPattern::V4Star),
            (IpAddr::V6(_), 0) => Ok(IpPattern::V6Star),
            (IpAddr::V4(addr_prefix), mask) if mask <= 32 => {
                Ok(IpPattern::V4 { addr_prefix, mask })
            }
            (IpAddr::V6(addr_prefix), mask) if mask <= 128 => {
                Ok(IpPattern::V6 { addr_prefix, mask })
            }
            (addr, mask) => {
                if addr.is_ipv4() {
                    Err(PolicyError::InvalidIpV4Mask { mask })
                } else {
                    Err(PolicyError::InvalidIpV6Mask { mask })
                }
            }
        }
    }

    /// Return true iff `addr` is matched by this pattern.
    fn matches(&self, addr: &IpAddr) -> bool {
        match (self, addr) {
            (IpPattern::Star, _) => true,
            (IpPattern::V4Star, IpAddr::V4(_)) => true,
            (IpPattern::V6Star, IpAddr::V6(_)) => true,
            (IpPattern::V4 { addr_prefix, mask }, IpAddr::V4(addr)) => {
                let p1 = u32::from_be_bytes(addr_prefix.octets());
                let p2 = u32::from_be_bytes(addr.octets());

                let shift = 32 - mask;
                (p1 >> shift) == (p2 >> shift)
            }
            (IpPattern::V6 { addr_prefix, mask }, IpAddr::V6(addr)) => {
                let p1 = u128::from_be_bytes(addr_prefix.octets());
                let p2 = u128::from_be_bytes(addr.octets());

                let shift = 128 - mask;
                (p1 >> shift) == (p2 >> shift)
            }
            (_, _) => false,
        }
    }
}

impl Display for IpPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpPattern::Star => write!(f, "*"),
            IpPattern::V4Star => write!(f, "*4"),
            IpPattern::V6Star => write!(f, "*6"),
            IpPattern::V4 { addr_prefix, mask } => {
                write!(f, "{addr_prefix}/{mask}")
            }
            IpPattern::V6 { addr_prefix, mask } => {
                write!(f, "{addr_prefix}/{mask}")
            }
        }
    }
}

/// Helper: try to parse a plain ipv4 address, or an IPv6 address
/// wrapped in brackets.
fn parse_addr(s: &str) -> Result<IpAddr, PolicyError> {
    if s.starts_with('[') && s.ends_with(']') {
        Ipv6Addr::from_str(&s[1..s.len() - 1]).map(IpAddr::V6)
    } else {
        IpAddr::from_str(s)
    }
    .map_err(|source| PolicyError::MalformedIpAddress {
        addr: s.to_string(),
        source,
    })
}

fn parse_port(s: &str) -> Result<u16, PolicyError> {
    s.parse::<u16>()
        .map_err(|_| PolicyError::InvalidPort { raw: s.to_string() })
}

impl FromStr for IpPattern {
    type Err = PolicyError;
    fn from_str(s: &str) -> Result<Self, PolicyError> {
        let (ip_s, mask_s) = match s.find('/') {
            Some(slash_idx) => (&s[..slash_idx], Some(&s[slash_idx + 1..])),
            None => (s, None),
        };

        match (ip_s, mask_s) {
            // '*' patterns
            ("*", Some(m)) => Err(PolicyError::MaskWithStar {
                mask: m.to_string(),
            }),
            ("*", None) => Ok(IpPattern::Star),

            // '*4' patterns
            ("*4", Some(m)) => Err(PolicyError::MaskWithV4Star {
                mask: m.to_string(),
            }),
            ("*4", None) => Ok(IpPattern::V4Star),

            // '*6' patterns
            ("*6", Some(m)) => Err(PolicyError::MaskWithV6Star {
                mask: m.to_string(),
            }),
            ("*6", None) => Ok(IpPattern::V6Star),

            (s, Some(m)) => {
                let a: IpAddr = parse_addr(s)?;
                let m: u8 = m.parse().map_err(|_| PolicyError::InvalidMask {
                    mask: m.to_string(),
                })?;
                IpPattern::from_addr_and_mask(a, m)
            }
            (s, None) => {
                let a: IpAddr = parse_addr(s)?;
                let m = if a.is_ipv4() { 32 } else { 128 };
                IpPattern::from_addr_and_mask(a, m)
            }
        }
    }
}

/// A PortRange is a set of consecutively numbered TCP or UDP ports.
///
/// # Example
/// ```
/// use nym_exit_policy::policy::PortRange;
///
/// let r: PortRange = "22-8000".parse().unwrap();
/// assert!(r.contains(128));
/// assert!(r.contains(22));
/// assert!(r.contains(8000));
///
/// assert!(! r.contains(21));
/// assert!(! r.contains(8001));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PortRange {
    /// The first port in this range.
    #[cfg_attr(feature = "openapi", schema(example = 80))]
    pub start: u16,

    /// The last port in this range.
    #[cfg_attr(feature = "openapi", schema(example = 81))]
    pub end: u16,
}

impl PortRange {
    /// Create a new port range spanning from start to end, asserting that
    /// the correct invariants hold.
    fn new_unchecked(start: u16, end: u16) -> Self {
        assert_ne!(start, 0);
        assert!(start <= end);

        PortRange { start, end }
    }

    /// Create a port range containing all ports.
    pub fn new_all() -> Self {
        PortRange::new_unchecked(1, 65535)
    }

    pub fn new_zero() -> Self {
        PortRange { start: 0, end: 0 }
    }

    /// Create a new PortRange.
    ///
    /// The Portrange contains all ports between `start` and `end` inclusive.
    ///
    /// Returns None if lo is greater than end, or if either is zero.
    pub const fn new(start: u16, end: u16) -> Option<Self> {
        if start != 0 && start <= end {
            Some(PortRange { start, end })
        } else {
            None
        }
    }

    /// Create a new singleton PortRange.
    pub const fn new_singleton(value: u16) -> Self {
        PortRange {
            start: value,
            end: value,
        }
    }

    /// Return true if a port is in this range.
    pub fn contains(&self, port: u16) -> bool {
        self.start <= port && port <= self.end
    }

    /// Return true if this range contains all ports.
    pub fn is_all(&self) -> bool {
        self.start == 1 && self.end == 65535
    }
}

/// A PortRange is displayed as a number if it contains a single port,
/// and as a start point and end point separated by a dash if it contains
/// more than one port.
impl Display for PortRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_all() {
            write!(f, "*")
        } else if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

impl FromStr for PortRange {
    type Err = PolicyError;
    fn from_str(s: &str) -> Result<Self, PolicyError> {
        // check is if it's a star range
        if s == "*" {
            return Ok(PortRange::new_all());
        }

        if let Some(pos) = s.find('-') {
            // This is a range; parse each part
            let start = parse_port(&s[..pos])?;
            let end = parse_port(&s[pos + 1..])?;
            PortRange::new(start, end).ok_or(PolicyError::InvalidRange { start, end })
        } else {
            // There was no hyphen, so try to parse this range as a singleton.
            let value = parse_port(s)?;
            Ok(PortRange::new_singleton(value))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bad_rules() {
        fn check(s: &str) {
            assert!(s.parse::<AddressPortPattern>().is_err());
        }

        check("marzipan:80");
        check("1.2.3.4:90-80");
        check("1.2.3.4:0-80");
        check("1.2.3.4/100:8888");
        check("[1.2.3.4]/16:80");
        check("[::1]/130:8888");
    }

    #[test]
    fn test_rule_matches() {
        fn check(address: &str, yes: &[&str], no: &[&str]) {
            use std::net::SocketAddr;
            let policy = address.parse::<AddressPortPattern>().unwrap();
            for s in yes {
                let sa = s.parse::<SocketAddr>().unwrap();
                assert!(policy.matches_sockaddr(&sa));
            }
            for s in no {
                let sa = s.parse::<SocketAddr>().unwrap();
                assert!(!policy.matches_sockaddr(&sa));
            }
        }

        check(
            "1.2.3.4/16:80",
            &["1.2.3.4:80", "1.2.44.55:80"],
            &["9.9.9.9:80", "1.3.3.4:80", "1.2.3.4:81"],
        );
        check(
            "*:443-8000",
            &["1.2.3.4:443", "[::1]:500"],
            &["9.0.0.0:80", "[::1]:80"],
        );
        check(
            "[face::]/8:80",
            &["[fab0::7]:80"],
            &["[dd00::]:80", "[face::7]:443"],
        );

        check("0.0.0.0/0:*", &["127.0.0.1:80"], &["[f00b::]:80"]);
        check("[::]/0:*", &["[f00b::]:80"], &["127.0.0.1:80"]);

        check(
            "*:0",
            &["1.2.3.4:0", "[::1]:0", "9.0.0.0:0"],
            &["1.2.3.4:443", "[::1]:500", "9.0.0.0:80", "[::1]:80"],
        );
        check(
            "*4:0",
            &["1.2.3.4:0", "9.0.0.0:0"],
            &["1.2.3.4:443", "9.0.0.0:80", "[::1]:0", "[::1]:80"],
        );
        check(
            "*6:0",
            &["[::1]:0"],
            &["[::1]:80", "1.2.3.4:0", "1.2.3.4:443"],
        );
    }

    #[test]
    fn test_policy_matches() -> Result<(), PolicyError> {
        let mut policy = AddressPolicy::default();
        policy.push(AddressPolicyAction::Accept, "*:443".parse()?);
        policy.push(AddressPolicyAction::Accept, "[::1]:80".parse()?);
        policy.push(AddressPolicyAction::Reject, "*:80".parse()?);
        policy.push(AddressPolicyAction::Accept, "*:0".parse()?);

        let policy = policy; // drop mut
        assert!(policy
            .allows_sockaddr(&"[::6]:443".parse().unwrap())
            .unwrap());
        assert!(policy
            .allows_sockaddr(&"127.0.0.1:443".parse().unwrap())
            .unwrap());
        assert!(policy
            .allows_sockaddr(&"[::1]:80".parse().unwrap())
            .unwrap());
        assert!(!policy
            .allows_sockaddr(&"[::2]:80".parse().unwrap())
            .unwrap());
        assert!(!policy
            .allows_sockaddr(&"127.0.0.1:80".parse().unwrap())
            .unwrap());
        assert!(policy
            .allows_sockaddr(&"127.0.0.1:66".parse().unwrap())
            .is_none());
        assert!(policy
            .allows_sockaddr(&"127.0.0.1:0".parse().unwrap())
            .unwrap());
        Ok(())
    }

    #[test]
    fn parse_portrange() {
        assert_eq!(
            "1-100".parse::<PortRange>().unwrap(),
            PortRange::new(1, 100).unwrap()
        );
        assert_eq!(
            "01-100".parse::<PortRange>().unwrap(),
            PortRange::new(1, 100).unwrap()
        );
        assert_eq!(
            "1-65535".parse::<PortRange>().unwrap(),
            PortRange::new_all()
        );
        assert_eq!(
            "10-30".parse::<PortRange>().unwrap(),
            PortRange::new(10, 30).unwrap()
        );
        assert_eq!(
            "9001".parse::<PortRange>().unwrap(),
            PortRange::new(9001, 9001).unwrap()
        );
        assert_eq!(
            "9001-9001".parse::<PortRange>().unwrap(),
            PortRange::new(9001, 9001).unwrap()
        );
        assert_eq!("*".parse::<PortRange>().unwrap(), PortRange::new_all());

        assert!("hello".parse::<PortRange>().is_err());
        assert!("65536".parse::<PortRange>().is_err());
        assert!("65537".parse::<PortRange>().is_err());
        assert!("1-2-3".parse::<PortRange>().is_err());
        assert!("10-5".parse::<PortRange>().is_err());
        assert!("1-".parse::<PortRange>().is_err());
        assert!("-2".parse::<PortRange>().is_err());
        assert!("-".parse::<PortRange>().is_err());

        assert_eq!("0".parse::<PortRange>().unwrap(), PortRange::new_zero(),);
        assert!("0-1".parse::<PortRange>().is_err());
    }

    #[test]
    fn test_portrange() {
        assert!(PortRange::new_all().is_all());
        assert!(!PortRange::new(2, 65535).unwrap().is_all());

        assert!(PortRange::new_all().contains(1));
        assert!(PortRange::new_all().contains(65535));
        assert!(PortRange::new_all().contains(7777));

        assert!(PortRange::new(20, 30).unwrap().contains(20));
        assert!(PortRange::new(20, 30).unwrap().contains(25));
        assert!(PortRange::new(20, 30).unwrap().contains(30));
        assert!(!PortRange::new(20, 30).unwrap().contains(19));
        assert!(!PortRange::new(20, 30).unwrap().contains(31));
    }

    // this test exists due to manually implemented 'stringified_ip_pattern' on 'AddressPortPattern'
    #[test]
    fn policy_serde_json_roundtrip() {
        let policy = AddressPolicy::parse_from_torrc(
            r#"
ExitPolicy reject 1.2.3.4/32:*
ExitPolicy reject 1.2.3.5:* 
ExitPolicy reject 1.2.3.6/16:*
ExitPolicy reject 1.2.3.6/16:123-456 
ExitPolicy accept *:53 
ExitPolicy accept6 *6:119
ExitPolicy accept *4:120
ExitPolicy reject6 [FC00::]/7:*
ExitPolicy reject FE80:0000:0000:0000:0202:B3FF:FE1E:8329:*
ExitPolicy reject FE80:0000:0000:0000:0202:B3FF:FE1E:8328:1234
ExitPolicy reject FE80:0000:0000:0000:0202:B3FF:FE1E:8328/64:1235
ExitPolicy reject *:*"#,
        )
        .unwrap();

        let json = serde_json::to_string(&policy).unwrap();
        let recovered: AddressPolicy = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered, policy);
    }
}
