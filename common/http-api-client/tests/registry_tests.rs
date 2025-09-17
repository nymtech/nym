use nym_http_api_client::registry;

// Create separate modules to avoid name conflicts
mod config_early {
    use nym_http_api_client_macro::client_defaults;

    client_defaults!(
        priority = -200;
        tcp_nodelay = true
    );
}

mod config_late {
    use nym_http_api_client_macro::client_defaults;

    client_defaults!(
        priority = 100;
        pool_idle_timeout = std::time::Duration::from_secs(90)
    );
}

#[test]
fn test_registry_collects_configs() {
    // Verify that configurations are being registered
    let count = registry::registered_config_count();
    // Should have at least the ones we registered above plus the default from lib.rs
    assert!(
        count >= 3,
        "Expected at least 3 registered configs, got {}",
        count
    );
}

#[test]
fn test_default_builder_applies_configs() {
    // Test that default_builder returns a configured builder
    let _builder = registry::default_builder();
    // The builder should have all configurations applied
    // We can't easily inspect the internals, but we verify it doesn't panic
}

#[test]
fn test_build_client_works() {
    // Test that we can successfully build a client with all configurations
    let result = registry::build_client();
    assert!(result.is_ok(), "Failed to build client: {:?}", result.err());
}

#[cfg(debug_assertions)]
#[test]
fn test_inspect_configs() {
    // In debug mode, test that we can inspect registered configurations
    let configs = registry::inspect_registered_configs();

    // Verify configs are sorted by priority
    for window in configs.windows(2) {
        assert!(window[0].0 <= window[1].0, "Configs not sorted by priority");
    }

    // Verify we have configs at different priority levels
    let priorities: Vec<i32> = configs.iter().map(|(p, _)| *p).collect();
    assert!(
        priorities.iter().any(|&p| p < 0),
        "Expected negative priority configs"
    );
    assert!(
        priorities.iter().any(|&p| p >= 0),
        "Expected non-negative priority configs"
    );
}
