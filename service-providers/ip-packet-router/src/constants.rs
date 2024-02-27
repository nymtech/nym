use std::time::Duration;

// The interface used to route traffic
pub const TUN_BASE_NAME: &str = "nymtun";
pub const TUN_DEVICE_ADDRESS_V4: &str = "10.0.0.1";
pub const TUN_DEVICE_NETMASK_V4: &str = "255.255.255.0";
pub const TUN_DEVICE_ADDRESS_V6: &str = "2001:db8:a160::1";
pub const TUN_DEVICE_NETMASK_V6: &str = "120";

// We routinely check if any clients needs to be disconnected at this interval
pub(crate) const DISCONNECT_TIMER_INTERVAL: Duration = Duration::from_secs(10);

// We consider a client inactive if it hasn't sent any mixnet packets in this duration
pub(crate) const CLIENT_MIXNET_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(5 * 60);

// We consider a client handler inactive if it hasn't received any packets from the tun device in
// this duration
pub(crate) const CLIENT_HANDLER_ACTIVITY_TIMEOUT: Duration = Duration::from_secs(10 * 60);
