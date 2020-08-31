use super::types::{AddrType, ResponseCode, SocksProxyError};
use super::{utils as socks_utils, SOCKS_VERSION};
use log::*;
use proxy_helpers::read_delay_loop::try_read_data;
use std::net::SocketAddr;
use tokio::prelude::*;

/// A Socks5 request hitting the proxy.
pub(crate) struct SocksRequest {
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

    /// Print out the address and port to a String.
    /// This might return domain:port, ipv6:port, or ipv4:port.
    pub(crate) fn to_string(&self) -> String {
        let address = socks_utils::pretty_print_addr(&self.addr_type, &self.addr);
        format!("{}:{}", address, self.port)
    }

    /// Convert the request object to a SocketAddr
    pub(crate) fn to_socket(&self) -> Result<Vec<SocketAddr>, SocksProxyError> {
        socks_utils::addr_to_socket(&self.addr_type, &self.addr, self.port)
    }

    /// Attempts to read data from the Socks5 request stream. Times out and
    /// returns what it's got if no data is read for the timeout_duration
    pub(crate) async fn try_read_request_data<R>(
        reader: &mut R,
        remote_address: &str,
    ) -> io::Result<(Vec<u8>, bool)>
    where
        R: AsyncRead + Unpin,
    {
        let timeout_duration = std::time::Duration::from_millis(500);
        try_read_data(timeout_duration, reader, remote_address).await
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
