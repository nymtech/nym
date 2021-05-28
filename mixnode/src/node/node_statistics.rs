use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

pub(crate) struct NodeStatsWrapper {
    inner: Arc<RwLock<NodeStats>>,
}

#[derive(Serialize)]
pub(crate) struct NodeStats {
    //
    packets_received_since_startup: AtomicUsize,
    packets_sent_since_startup: AtomicUsize,
    dropped_packets_since_startup: HashMap<String, usize>,

    // perhaps serialize as unix nanos to keep it atomic?
    checkpoint: SystemTime,
    packets_received_since_last_checkpoint: AtomicUsize,
    packets_sent_since_last_checkpoint: AtomicUsize,
}
