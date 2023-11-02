# Manually Handled Storage
If you're integrating mixnet functionality into an existing app and want to integrate saving client configs and keys into your existing storage logic, you can manually perform the actions taken automatically above (`examples/manually_handle_keys_and_config.rs`)

```rust,noplayground
{{#include ../../../../../../sdk/rust/nym-sdk/examples/manually_handle_storage.rs}}
```
