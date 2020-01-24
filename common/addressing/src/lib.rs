use std::convert::{TryFrom, TryInto};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub enum AddressType {
    V4,
    V6,
}

impl Into<u8> for AddressType {
    fn into(self) -> u8 {
        use AddressType::*;

        match self {
            V4 => 4,
            V6 => 6,
        }
    }
}

#[derive(Debug)]
pub enum AddressTypeError {
    InvalidPrefixError,
}

impl TryFrom<u8> for AddressType {
    type Error = AddressTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use AddressType::*;
        use AddressTypeError::*;

        match value {
            4 => Ok(V4),
            6 => Ok(V6),
            _ => Err(InvalidPrefixError),
        }
    }
}

/// FLAG || port || octets || zeropad
pub fn encoded_bytes_from_socket_address(address: SocketAddr) -> [u8; 32] {
    let port_bytes = address.port().to_be_bytes();

    let encoded_host: Vec<u8> = match address.ip() {
        IpAddr::V4(ip) => std::iter::once(AddressType::V4.into())
            .chain(port_bytes.iter().cloned())
            .chain(ip.octets().iter().cloned())
            .chain(std::iter::repeat(0))
            .take(32)
            .collect(),
        IpAddr::V6(ip) => std::iter::once(AddressType::V6.into())
            .chain(port_bytes.iter().cloned())
            .chain(ip.octets().iter().cloned())
            .chain(std::iter::repeat(0))
            .take(32)
            .collect(),
    };

    let mut address_bytes = [0u8; 32];
    address_bytes.copy_from_slice(&encoded_host[..32]);

    address_bytes
}

pub fn socket_address_from_encoded_bytes(b: [u8; 32]) -> Result<SocketAddr, AddressTypeError> {
    let address_type: AddressType = b[0].try_into()?;

    let port: u16 = u16::from_be_bytes([b[1], b[2]]);

    let ip = match address_type {
        AddressType::V4 => IpAddr::V4(Ipv4Addr::new(b[3], b[4], b[5], b[6])),
        AddressType::V6 => {
            let mut address_octets = [0u8; 16];
            address_octets.copy_from_slice(&b[3..19]);
            IpAddr::V6(Ipv6Addr::from(address_octets))
        }
    };

    Ok(SocketAddr::new(ip, port))
}
