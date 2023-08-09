use crate::socks::SOCKS4_VERSION;

use super::types::{AddrType, ResponseCodeV5, SocksProxyError};
use super::{utils as socks_utils, SOCKS5_VERSION};
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
    /// Parse a SOCKS4 request from a `TcpStream`
    /// From documents at:
    ///  - SOCKS4: https://www.openssh.com/txt/socks4.protocol
    ///  - SOCKS4a: https://www.openssh.com/txt/socks4a.protocol
    pub async fn from_stream_socks4<R>(stream: &mut R) -> Result<Self, SocksProxyError>
    where
        R: AsyncRead + Unpin,
    {
        log::trace!("read from stream socks4");

        let mut packet = [0u8; 3];
        stream
            .read_exact(&mut packet)
            .await
            .map_err(|source| SocksProxyError::SocketReadError { source })?;

        // CD (command)
        let Some(command) = SocksCommand::from(packet[0] as usize) else {
            log::warn!("Invalid Command");
            return Err(ResponseCodeV5::CommandNotSupported.into());
        };

        // DSTPORT
        let mut port = [0u8; 2];
        port.copy_from_slice(&packet[1..]);
        let port = merge_u8_into_u16(port[0], port[1]);

        // DSTIP
        let mut ip = [0u8; 4];
        stream
            .read_exact(&mut ip)
            .await
            .map_err(|source| SocksProxyError::SocketReadError { source })?;

        // USERID
        let _userid = read_until_zero(stream).await;

        // SOCKS4a extension
        // https://www.openssh.com/txt/socks4a.protocol
        // If the IP is 0.0.0.x with x nonzero, read the domain name
        let (addr, addr_type) = if ip[..3] == [0, 0, 0] && ip[3] != 0 {
            (read_until_zero(stream).await?, AddrType::Domain)
        } else {
            (ip.to_vec(), AddrType::V4)
        };

        // Return parsed request
        Ok(SocksRequest {
            version: SOCKS4_VERSION,
            command,
            addr_type,
            addr,
            port,
        })
    }
    /// Parse a SOCKS5 request from a `TcpStream`
    /// From: https://www.rfc-editor.org/rfc/rfc1928
    pub async fn from_stream_socks5<R>(stream: &mut R) -> Result<Self, SocksProxyError>
    where
        R: AsyncRead + Unpin,
    {
        log::trace!("read from stream socks5");

        let mut packet = [0u8; 4];
        // Read a byte from the stream and determine the version being requested
        stream
            .read_exact(&mut packet)
            .await
            .map_err(|source| SocksProxyError::SocketReadError { source })?;

        // VER
        if packet[0] != SOCKS5_VERSION {
            warn!("Unsupported version: SOCKS{}", packet[0]);
            return Err(SocksProxyError::UnsupportedProxyVersion {
                version: (packet[0]),
            });
        }

        // CMD
        let Some(command) = SocksCommand::from(packet[1] as usize) else {
            warn!("Invalid Command");
            return Err(ResponseCodeV5::CommandNotSupported.into());
        };

        // RSV
        // packet[2] is reserved

        // ATYP
        let Some(addr_type) = AddrType::from(packet[3] as usize) else {
            error!("No Addr");
            return Err(ResponseCodeV5::AddrTypeNotSupported.into());
        };

        // DST.ADDR
        let addr = match addr_type {
            AddrType::Domain => {
                let mut domain_length = [0u8];
                stream
                    .read_exact(&mut domain_length)
                    .await
                    .map_err(|source| SocksProxyError::SocketReadError { source })?;
                let mut domain = vec![0u8; domain_length[0] as usize];
                stream
                    .read_exact(&mut domain)
                    .await
                    .map_err(|source| SocksProxyError::SocketReadError { source })?;
                domain
            }
            AddrType::V4 => {
                let mut addr = [0u8; 4];
                stream
                    .read_exact(&mut addr)
                    .await
                    .map_err(|source| SocksProxyError::SocketReadError { source })?;
                addr.to_vec()
            }
            AddrType::V6 => {
                let mut addr = [0u8; 16];
                stream
                    .read_exact(&mut addr)
                    .await
                    .map_err(|source| SocksProxyError::SocketReadError { source })?;
                addr.to_vec()
            }
        };

        // DST.PORT
        let mut port = [0u8; 2];
        stream
            .read_exact(&mut port)
            .await
            .map_err(|source| SocksProxyError::SocketReadError { source })?;
        let port = merge_u8_into_u16(port[0], port[1]);

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
    pub fn address_string(&self) -> String {
        let address = socks_utils::pretty_print_addr(&self.addr_type, &self.addr);
        format!("{}:{}", address, self.port)
    }
}

impl Display for SocksRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.address_string())
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

fn merge_u8_into_u16(a: u8, b: u8) -> u16 {
    (u16::from(a) << 8) | u16::from(b)
}

async fn read_until_zero<R>(stream: &mut R) -> Result<Vec<u8>, SocksProxyError>
where
    R: AsyncRead + Unpin,
{
    let mut result = Vec::new();
    let mut char = [0u8];
    loop {
        stream
            .read_exact(&mut char)
            .await
            .map_err(|source| SocksProxyError::SocketReadError { source })?;
        if char[0] == 0 {
            break;
        }
        result.push(char[0]);
    }
    Ok(result)
}
