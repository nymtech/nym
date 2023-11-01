use std::fmt::{Display, Formatter};

use bytes::Bytes;

#[allow(unused)]
#[derive(Debug)]
pub enum Event {
    /// IP packet received from the WireGuard tunnel that should be passed through to the
    /// corresponding virtual device/internet.
    Wg(Bytes),
    /// IP packet received from the WireGuard tunnel that was verified as part of the handshake.
    WgVerified(Bytes),
    /// IP packet to be sent through the WireGuard tunnel as crafted by the virtual device.
    Ip(Bytes),
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Wg(data) => {
                let size = data.len();
                write!(f, "Wg{{ size={size} }}")
            }
            Event::WgVerified(data) => {
                let size = data.len();
                write!(f, "WgVerified{{ size={size} }}")
            }
            Event::Ip(data) => {
                let size = data.len();
                write!(f, "Ip{{ size={size} }}")
            }
        }
    }
}
