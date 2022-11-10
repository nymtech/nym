use std::sync::atomic::AtomicBool;

pub mod cover_traffic_stream;
pub mod inbound_messages;
pub mod key_manager;
pub mod mix_traffic;
pub mod real_messages_control;
pub mod received_buffer;
pub mod replies;
pub mod topology_control;

// This is *NOT* used to signal shutdown.
// It's critical that we don't have any tasks finishing early, this is an additional safety check
// that tasks exiting are doing so because shutdown has been signalled, and no other reason.
// In particular for tasks that rely on their associated channel being closed to signal shutdown,
// and don't have access to a shutdown listener channel.
pub static SHUTDOWN_HAS_BEEN_SIGNALLED: AtomicBool = AtomicBool::new(false);
