use std::sync::atomic::AtomicBool;

pub mod cover_traffic_stream;
pub mod inbound_messages;
pub mod key_manager;
pub mod mix_traffic;
pub mod real_messages_control;
pub mod received_buffer;
pub mod reply_key_storage;
pub mod topology_control;

// This is *NOT* used to signal shutdown, it's used to assert that tasks finishing are doing so
// because shutdown has been signalled.
// In particular for tasks that rely on their associated channel being closed to signal shutdown.
pub static SHUTDOWN_HAS_BEEN_SIGNALLED: AtomicBool = AtomicBool::new(false);
