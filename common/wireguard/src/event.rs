use std::fmt::{Display, Formatter};

use bytes::Bytes;

#[allow(unused)]
#[derive(Debug)]
pub enum Event {
    /// IP packet received from the WireGuard tunnel that should be passed through to the corresponding virtual device/internet.
    /// Original implementation also has protocol here since it understands it, but we'll have to infer it downstream
    Wg(Bytes),
    /// IP packet received from the UDP listener that was verified as part of the handshake
    WgVerified(Bytes),
    /// IP packet to be sent through the WireGuard tunnel as crafted by the virtual device.
    Ip(Bytes),
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Wg(data) => {
                let size = data.len();
                write!(f, "WgPacket{{ size={size} }}")
            }
            Event::WgVerified(data) => {
                let size = data.len();
                write!(f, "WgVerifiedPacket{{ size={size} }}")
            }
            Event::Ip(data) => {
                let size = data.len();
                write!(f, "IpPacket{{ size={size} }}")
            }
        }
    }
}
