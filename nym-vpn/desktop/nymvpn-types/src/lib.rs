pub mod device;
pub mod location;
pub mod notification;
pub mod nymvpn_server;
pub mod vpn_session;
pub mod wireguard;

pub type DateTimeUtc = chrono::DateTime<chrono::Utc>;

#[cfg(target_os = "linux")]
pub const TUNNEL_TABLE_ID: u32 = 0x686c6565;
#[cfg(target_os = "linux")]
pub const TUNNEL_FWMARK: u32 = 0x686c6565;
