use std::fmt::{Display, Formatter};

use bytes::Bytes;

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Event {
    /// Dumb event with no data.
    Dumb,
    /// IP packet received from the WireGuard tunnel that should be passed through to the corresponding virtual device/internet.
    /// Original implementation also has protocol here since it understands it, but we'll have to infer it downstream
    WgPacket(Bytes),
    /// IP packet to be sent through the WireGuard tunnel as crafted by the virtual device.
    IpPacket(Bytes),
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Dumb => {
                write!(f, "Dumb{{}}")
            }
            Event::WgPacket(data) => {
                let size = data.len();
                write!(f, "WgPacket{{ size={size} }}")
            }
            Event::IpPacket(data) => {
                let size = data.len();
                write!(f, "IpPacket{{ size={size} }}")
            }
        }
    }
}
