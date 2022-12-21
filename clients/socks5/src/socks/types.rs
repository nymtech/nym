/// SOCKS4 Response codes
#[allow(dead_code)]
pub(crate) enum ResponseCodeV4 {
    Granted = 0x5a,
    RequestRejected = 0x5b,
    CannotConnectToIdent = 0x5c,
    DifferentUserId = 0x5d,
}

/// Possible SOCKS5 Response Codes
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum ResponseCodeV5 {
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

#[derive(Debug)]
pub enum SocksProxyError {
    GenericError(Box<dyn std::error::Error + Send + Sync>),
    UnsupportedProxyVersion(u8),
}

impl std::fmt::Display for SocksProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocksProxyError::GenericError(err) => write!(f, "GenericError - {}", err),
            SocksProxyError::UnsupportedProxyVersion(version) => {
                write!(f, "Unsupported proxy version {}", version)
            }
        }
    }
}

impl<E> From<E> for SocksProxyError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        SocksProxyError::GenericError(Box::new(err))
    }
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
