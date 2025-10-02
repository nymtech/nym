use opentelemetry_sdk::trace::IdGenerator;
use opentelemetry::trace::{TraceId, SpanId};
use rand::RngCore;

#[derive(Clone, Debug)]
pub struct Compact13BytesIdGenerator;

impl IdGenerator for Compact13BytesIdGenerator {
    fn new_trace_id(&self) -> TraceId {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 16];

        // Fill the first 13 bytes with random data
        rng.fill_bytes(&mut bytes[0..13]);
        // Set the last 3 bytes to zero
        bytes[13] = 0;
        bytes[14] = 0;
        bytes[15] = 0;

        TraceId::from_bytes(bytes)
    }

    fn new_span_id(&self) -> SpanId {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 8];
        rng.fill_bytes(&mut bytes);
        
        SpanId::from_bytes(bytes)
    }
}

pub fn compress_trace_id(trace_id: &TraceId) -> [u8; 13] {
    let bytes = trace_id.to_bytes();

    let mut compressed = [0u8; 13];
    compressed.copy_from_slice(&bytes[0..13]);

    compressed
}

pub fn decompress_trace_id(compressed: &[u8; 13]) -> TraceId {
    let mut bytes = [0u8; 16];
    bytes[0..13].copy_from_slice(compressed);

    TraceId::from_bytes(bytes)
}