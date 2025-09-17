use nym_http_api_client_macro::{client_cfg, client_defaults};
use std::time::Duration;

#[test]
fn test_client_cfg_basic() {
    // Test that the macro compiles with basic configuration
    let _config = client_cfg!(timeout = Duration::from_secs(30), gzip = true);
}

#[test]
fn test_client_cfg_with_headers() {
    // Test that the macro compiles with default headers
    let _config = client_cfg!(
        timeout = Duration::from_secs(30),
        default_headers {
            "User-Agent" => "TestApp/1.0",
            "Accept" => "application/json"
        }
    );
}

#[test]
fn test_client_cfg_with_method_calls() {
    // Test that the macro compiles with method calls
    let _config = client_cfg!(
        pool_max_idle_per_host = 32,
        tcp_nodelay = true,
        danger_accept_invalid_certs = true
    );
}

#[test]
fn test_client_defaults_with_priority() {
    // Test that client_defaults macro compiles with priority
    client_defaults!(
        priority = -100;
        gzip = true,
        deflate = true
    );
}

#[test]
fn test_client_defaults_without_priority() {
    // Test that client_defaults macro compiles without priority (defaults to 0)
    client_defaults!(brotli = true, zstd = true);
}

#[test]
fn test_empty_client_cfg() {
    // Test that empty configuration compiles
    let _config = client_cfg!();
}

// Integration test to verify the closure actually works
#[test]
fn test_client_cfg_closure_application() {
    let config = client_cfg!(gzip = true);

    // Apply the configuration to a new builder
    let builder = reqwest::ClientBuilder::new();
    let _configured_builder = config(builder);
    // Note: We can't easily test the internal state of the builder,
    // but we verify it compiles and runs without panic
}
