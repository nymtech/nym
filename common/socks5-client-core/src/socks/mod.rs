#![forbid(unsafe_code)]



use self::types::SocksProxyError;

pub mod authentication;
pub(crate) mod client;
pub(crate) mod mixnet_responses;
mod request;
pub mod server;
pub mod types;
pub mod utils;

/// Version of socks
const SOCKS4_VERSION: u8 = 0x04;
const SOCKS5_VERSION: u8 = 0x05;

const RESERVED: u8 = 0x00;

#[derive(Clone, PartialEq, Eq)]
pub enum SocksVersion {
    V4 = 0x04,
    V5 = 0x05,
}

pub struct InvalidSocksVersion;

impl TryFrom<u8> for SocksVersion {
    type Error = SocksProxyError;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            SOCKS4_VERSION => Ok(Self::V4),
            SOCKS5_VERSION => Ok(Self::V5),
            _ => Err(SocksProxyError::UnsupportedProxyVersion { version }),
        }
    }
}
