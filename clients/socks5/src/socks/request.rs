use super::{utils, AddrType, ResponseCode, SocksProxyError, SOCKS_VERSION};
use log::*;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::prelude::*;

/// Proxy User Request
pub(crate) struct SocksRequest {
    pub version: u8,
    pub command: SocksCommand,
    pub addr_type: AddrType,
    pub addr: Vec<u8>,
    pub port: u16,
}

impl SocksRequest {
    /// Parse a SOCKS Req from a TcpStream
    pub async fn from_stream(stream: &mut TcpStream) -> Result<Self, SocksProxyError> {
        let mut packet = [0u8; 4];
        // Read a byte from the stream and determine the version being requested
        stream.read_exact(&mut packet).await?;

        if packet[0] != SOCKS_VERSION {
            warn!("from_stream Unsupported version: SOCKS{}", packet[0]);
            stream.shutdown();
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
                stream.shutdown();
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
                stream.shutdown();
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

    pub(crate) fn to_string(&self) -> String {
        let address = utils::pretty_print_addr(&self.addr_type, &self.addr);
        format!("{}:{}", address, self.port)
    }

    pub(crate) fn to_socket(&self) -> Result<Vec<SocketAddr>, SocksProxyError> {
        utils::addr_to_socket(&self.addr_type, &self.addr, self.port)
    }

    /// Serialize the destination address and port (as a string) concatenated with
    /// the entirety of the request stream. Return it all as a sequence of bytes.
    pub async fn serialize(&mut self, stream: &mut TcpStream) -> Vec<u8> {
        let remote_address = self.to_string();
        let remote_bytes = remote_address.into_bytes();
        let remote_bytes_len = remote_bytes.len() as u16;
        let temp_bytes = remote_bytes_len.to_be_bytes(); // this is [u8; 2];
        let mut buf = temp_bytes
            .iter()
            .cloned()
            .chain(remote_bytes.into_iter())
            .collect::<Vec<_>>();

        stream.read_to_end(&mut buf).await.unwrap(); // appends the rest of the request stream into buf
        buf
    }
}

/// SOCK5 CMD Type
#[derive(Debug)]
pub(crate) enum SocksCommand {
    Connect = 0x01,
    Bind = 0x02,
    UdpAssociate = 0x3,
}

impl SocksCommand {
    /// Parse Byte to Command
    fn from(n: usize) -> Option<SocksCommand> {
        match n {
            1 => Some(SocksCommand::Connect),
            2 => Some(SocksCommand::Bind),
            3 => Some(SocksCommand::UdpAssociate),
            _ => None,
        }
    }
}
