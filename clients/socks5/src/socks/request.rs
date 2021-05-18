use super::types::{AddrType, ResponseCode, SocksProxyError};
use super::{utils as socks_utils, SOCKS_VERSION};
use log::*;
use std::fmt::{self, Display};
use tokio::io::{AsyncRead, AsyncReadExt};

/// A Socks5 request hitting the proxy.
pub(crate) struct SocksRequest {
    #[allow(dead_code)]
    pub version: u8,
    pub command: SocksCommand,
    pub addr_type: AddrType,
    pub addr: Vec<u8>,
    pub port: u16,
}

impl SocksRequest {
    /// Parse a SOCKS5 request from a TcpStream
    pub async fn from_stream<R>(stream: &mut R) -> Result<Self, SocksProxyError>
    where
        R: AsyncRead + Unpin,
    {
        let mut packet = [0u8; 4];
        // Read a byte from the stream and determine the version being requested
        stream.read_exact(&mut packet).await?;

        if packet[0] != SOCKS_VERSION {
            warn!("from_stream Unsupported version: SOCKS{}", packet[0]);
            return Err(SocksProxyError::UnsupportedProxyVersion(packet[0]));
        }

        // Get command
        let mut command: SocksCommand = SocksCommand::Connect;
        match SocksCommand::from(packet[1] as usize) {
            Some(com) => {
                command = com;
                Ok(())
            }
            None => {
                warn!("Invalid Command");
                Err(ResponseCode::CommandNotSupported)
            }
        }?;

        // DST.address

        let mut addr_type: AddrType = AddrType::V6;
        match AddrType::from(packet[3] as usize) {
            Some(addr) => {
                addr_type = addr;
                Ok(())
            }
            None => {
                error!("No Addr");
                Err(ResponseCode::AddrTypeNotSupported)
            }
        }?;

        trace!("Getting Addr");
        // Get Addr from addr_type and stream
        let addr: Result<Vec<u8>, SocksProxyError> = match addr_type {
            AddrType::Domain => {
                let mut domain_length = [0u8; 1];
                stream.read_exact(&mut domain_length).await?;

                let mut domain = vec![0u8; domain_length[0] as usize];
                stream.read_exact(&mut domain).await?;

                Ok(domain)
            }
            AddrType::V4 => {
                let mut addr = [0u8; 4];
                stream.read_exact(&mut addr).await?;
                Ok(addr.to_vec())
            }
            AddrType::V6 => {
                let mut addr = [0u8; 16];
                stream.read_exact(&mut addr).await?;
                Ok(addr.to_vec())
            }
        };

        let addr = addr?;

        // read DST.port
        let mut port = [0u8; 2];
        stream.read_exact(&mut port).await?;
        // Merge two u8s into u16
        let port = (u16::from(port[0]) << 8) | u16::from(port[1]);

        // Return parsed request
        Ok(SocksRequest {
            version: packet[0],
            command,
            addr_type,
            addr,
            port,
        })
    }
}

impl Display for SocksRequest {
    /// Print out the address and port to a String.
    /// This might return domain:port, ipv6:port, or ipv4:port.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let address = socks_utils::pretty_print_addr(&self.addr_type, &self.addr);
        write!(f, "{}:{}", address, self.port)
    }
}

/// SOCK5 CMD type
#[derive(Debug)]
pub(crate) enum SocksCommand {
    Connect = 0x01,
    Bind = 0x02,
    UdpAssociate = 0x3,
}

impl SocksCommand {
    /// Parse bytes to SocksCommand
    fn from(n: usize) -> Option<SocksCommand> {
        match n {
            1 => Some(SocksCommand::Connect),
            2 => Some(SocksCommand::Bind),
            3 => Some(SocksCommand::UdpAssociate),
            _ => None,
        }
    }
}
