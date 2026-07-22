#[cfg(target_os = "linux")]
mod linux;

pub mod tun_task_channel;

#[cfg(target_os = "linux")]
pub use linux::tun_device;

/// Default IPR egress TUN MTU (bytes), also reported to v10 clients as the
/// largest IP packet the IPR will accept.
pub const DEFAULT_IPR_TUN_MTU: u16 = 1500;

/// The IPR TUN MTU: `NYM_MTU_SIZE` if set and valid, else [`DEFAULT_IPR_TUN_MTU`].
/// Single source so the TUN and the value reported to clients agree.
pub fn configured_ipr_tun_mtu() -> u16 {
    std::env::var("NYM_MTU_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        // Reject sub-IPv6-minimum overrides (e.g. 0), which would break the TUN.
        .filter(|&mtu| mtu >= 1280)
        .unwrap_or(DEFAULT_IPR_TUN_MTU)
}
