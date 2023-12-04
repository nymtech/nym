use std::time::Duration;

// The interface used to route traffic
pub const TUN_BASE_NAME: &str = "nymtun";
pub const TUN_DEVICE_ADDRESS: &str = "10.0.0.1";
pub const TUN_DEVICE_NETMASK: &str = "255.255.255.0";

pub(crate) const DISCONNECT_TIMER_INTERVAL: Duration = Duration::from_secs(10);
pub(crate) const CLIENT_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(5 * 60);
