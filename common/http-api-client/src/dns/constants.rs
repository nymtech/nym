#![allow(missing_docs)]
#![allow(unused)]

use crate::dns::ResolveError;

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};

pub const NYM_API_DOMAIN: &str = "validator.nymtech.net";
pub const NYM_API_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(212, 71, 233, 232))];

pub const NYM_VPN_API_DOMAIN: &str = "nymvpn.com";
pub const NYM_VPN_API_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(76, 76, 21, 21))];

pub const NYM_FRONTDOOR_VERCEL_DOMAIN: &str = "nym-frontdoor.vercel.app";
pub const NYM_FRONTDOOR_VERCEL_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(64, 29, 17, 195)),
    IpAddr::V4(Ipv4Addr::new(216, 198, 79, 195)),
];

pub const NYM_FRONTDOOR_FASTLY_DOMAIN: &str = "nym-frontdoor.global.ssl.fastly.net";
pub const NYM_FRONTDOOR_FASTLY_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(151, 101, 193, 194)),
    IpAddr::V4(Ipv4Addr::new(151, 101, 129, 194)),
    IpAddr::V4(Ipv4Addr::new(151, 101, 1, 194)),
    IpAddr::V4(Ipv4Addr::new(151, 101, 65, 194)),
];

pub const NYMVPN_FRONTDOOR_FASTLY_DOMAIN: &str = "nymvpn-frontdoor.global.ssl.fastly.net";
pub const NYMVPN_FRONTDOOR_FASTLY_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(151, 101, 193, 194)),
    IpAddr::V4(Ipv4Addr::new(151, 101, 129, 194)),
    IpAddr::V4(Ipv4Addr::new(151, 101, 1, 194)),
    IpAddr::V4(Ipv4Addr::new(151, 101, 65, 194)),
];

pub const VERCEL_APP_DOMAIN: &str = "vercel.app";
pub const VERCEL_APP_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(64, 29, 17, 195)),
    IpAddr::V4(Ipv4Addr::new(216, 198, 79, 195)),
];

pub const VERCEL_COM_DOMAIN: &str = "vercel.com";
pub const VERCEL_COM_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(198, 169, 2, 129)),
    IpAddr::V4(Ipv4Addr::new(198, 169, 1, 193)),
];

pub const NYM_COM_DOMAIN: &str = "nym.com";
pub const NYM_COM_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(76, 76, 21, 22))];

pub const NYM_STATS_API_DOMAIN: &str = "nym-statistics-api.nymtech.cc";
pub const NYM_STATS_API_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(91, 92, 153, 96))];


lazy_static! {
    pub static ref DEFAULT_STATIC_ADDRS: HashMap<String, Vec<IpAddr>> = {
        let mut m = HashMap::new();
        m.insert(NYM_API_DOMAIN.to_string(), NYM_API_IPS.to_vec());
        m.insert(NYM_VPN_API_DOMAIN.to_string(), NYM_VPN_API_IPS.to_vec());
        m.insert(
            NYM_FRONTDOOR_VERCEL_DOMAIN.to_string(),
            NYM_FRONTDOOR_VERCEL_IPS.to_vec(),
        );
        m.insert(
            NYM_FRONTDOOR_FASTLY_DOMAIN.to_string(),
            NYM_FRONTDOOR_FASTLY_IPS.to_vec(),
        );
        m.insert(
            NYMVPN_FRONTDOOR_FASTLY_DOMAIN.to_string(),
            NYMVPN_FRONTDOOR_FASTLY_IPS.to_vec(),
        );
        m.insert(VERCEL_APP_DOMAIN.to_string(), VERCEL_APP_IPS.to_vec());
        m.insert(VERCEL_COM_DOMAIN.to_string(), VERCEL_COM_IPS.to_vec());
        m.insert(NYM_COM_DOMAIN.to_string(), NYM_COM_IPS.to_vec());
        m.insert(NYM_STATS_API_DOMAIN.to_string(), NYM_STATS_API_IPS.to_vec());
        m
    };
}

pub fn default_static_addrs() -> HashMap<String, Vec<IpAddr>> {
    let mut m = HashMap::new();
    m.insert(NYM_API_DOMAIN.to_string(), NYM_API_IPS.to_vec());
    m.insert(NYM_VPN_API_DOMAIN.to_string(), NYM_VPN_API_IPS.to_vec());
    m.insert(
        NYM_FRONTDOOR_VERCEL_DOMAIN.to_string(),
        NYM_FRONTDOOR_VERCEL_IPS.to_vec(),
    );
    m.insert(
        NYM_FRONTDOOR_FASTLY_DOMAIN.to_string(),
        NYM_FRONTDOOR_FASTLY_IPS.to_vec(),
    );
    m.insert(
        NYMVPN_FRONTDOOR_FASTLY_DOMAIN.to_string(),
        NYMVPN_FRONTDOOR_FASTLY_IPS.to_vec(),
    );
    m.insert(VERCEL_APP_DOMAIN.to_string(), VERCEL_APP_IPS.to_vec());
    m.insert(VERCEL_COM_DOMAIN.to_string(), VERCEL_COM_IPS.to_vec());
    m.insert(NYM_COM_DOMAIN.to_string(), NYM_COM_IPS.to_vec());
    m
}
