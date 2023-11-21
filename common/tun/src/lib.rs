#[cfg(target_os = "linux")]
mod linux;

pub mod tun_task_channel;

#[cfg(target_os = "linux")]
pub use linux::tun_device;
