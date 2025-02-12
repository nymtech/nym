use tower_http::compression::CompressionLayer;

/// Returns a tower-http `Layer` that manages optional compression for user responses. This should
/// respect preferences in user Accept-Encoding headers.
///
/// The default condition is to compress responses unless:
/// * They’re gRPC, which has its own protocol specific compression scheme.
/// * It’s an image as determined by the content-type starting with image/.
/// * They’re Server-Sent Events (SSE) as determined by the content-type being text/event-stream.
/// * The response is less than 32 bytes.
pub fn new_compression_layer() -> CompressionLayer {
    CompressionLayer::new()
        .br(true)
        .deflate(true)
        .gzip(true)
        .zstd(true)
}
