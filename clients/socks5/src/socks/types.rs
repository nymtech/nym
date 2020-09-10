use snafu::Snafu;
#[derive(Debug, Snafu)]
/// Possible SOCKS5 Response Codes
pub(crate) enum ResponseCode {
    Success = 0x00,
    #[snafu(display("SOCKS5 Server Failure"))]
    Failure = 0x01,
    #[snafu(display("SOCKS5 Rule failure"))]
    RuleFailure = 0x02,
    #[snafu(display("network unreachable"))]
    NetworkUnreachable = 0x03,
    #[snafu(display("host unreachable"))]
    HostUnreachable = 0x04,
    #[snafu(display("connection refused"))]
    ConnectionRefused = 0x05,
    #[snafu(display("TTL expired"))]
    TtlExpired = 0x06,
    #[snafu(display("Command not supported"))]
    CommandNotSupported = 0x07,
    #[snafu(display("Addr Type not supported"))]
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
#[derive(PartialEq)]
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
