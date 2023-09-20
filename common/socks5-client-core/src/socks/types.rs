use nym_socks5_requests::Socks5RequestError;
use std::string::FromUtf8Error;
use thiserror::Error;

/// SOCKS4 Response codes
#[allow(dead_code)]
pub(crate) enum ResponseCodeV4 {
    Granted = 0x5a,
    RequestRejected = 0x5b,
    CannotConnectToIdent = 0x5c,
    DifferentUserId = 0x5d,
}

/// Possible SOCKS5 Response Codes
#[derive(Debug, Error)]
pub enum ResponseCodeV5 {
    #[error("SOCKS5 Server Success")]
    Success = 0x00,
    #[error("SOCKS5 Server Failure")]
    Failure = 0x01,
    #[error("SOCKS5 Rule failure")]
    RuleFailure = 0x02,
    #[error("network unreachable")]
    NetworkUnreachable = 0x03,
    #[error("host unreachable")]
    HostUnreachable = 0x04,
    #[error("connection refused")]
    ConnectionRefused = 0x05,
    #[error("TTL expired")]
    TtlExpired = 0x06,
    #[error("Command not supported")]
    CommandNotSupported = 0x07,
    #[error("Addr Type not supported")]
    AddrTypeNotSupported = 0x08,
}

#[derive(Error, Debug)]
pub enum SocksProxyError {
    #[error("{version} of the socks protocol is not supported by this client")]
    UnsupportedProxyVersion { version: u8 },

    #[error("failed to write to the socket: {source}")]
    SocketWriteError {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read from the socket: {source}")]
    SocketReadError {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to shutdown underlying socket stream: {source}")]
    SocketShutdownFailure {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to extract ip address of the connected peer: {source}")]
    PeerAddrExtractionFailure {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to authenticate user due to malformed username: {source}")]
    MalformedAuthUsername {
        #[source]
        source: FromUtf8Error,
    },

    #[error("failed to authenticate user due to malformed password: {source}")]
    MalformedAuthPassword {
        #[source]
        source: FromUtf8Error,
    },

    #[error(transparent)]
    Socks5ResponseFailure(#[from] ResponseCodeV5),

    #[error("could not complete the provider request: {source}")]
    ProviderRequestFailure {
        #[from]
        source: Socks5RequestError,
    },

    #[error("SOCKS5 UDP not (yet) supported")]
    UdpNotSupported,

    #[error("SOCKS5 BIND not (yet) supported")]
    BindNotSupported,
}

/// DST.addr variant types
#[derive(Debug, PartialEq)]
pub(crate) enum AddrType {
    V4 = 0x01,
    Domain = 0x03,
    V6 = 0x04,
}

impl AddrType {
    /// Parse Byte to Command
    pub(crate) fn from(n: usize) -> Option<AddrType> {
        match n {
            1 => Some(AddrType::V4),
            3 => Some(AddrType::Domain),
            4 => Some(AddrType::V6),
            _ => None,
        }
    }
}
