---
title: Install the Nym Rust SDK
description: Add nym-sdk to your Rust project from Git or crates.io. Covers version requirements, minimum Rust version, and current feature gate status.
url: https://nym.com/docs/developers/rust/importing
---

# Installation

```toml
[dependencies]
nym-sdk = "1.21.5-rc.3"
```

**Minimum Rust version:** {RUST_MSRV}+

### From Git

You can also import directly from Git if you want unreleased changes:

```toml
# development branch (latest changes, may be unstable)
nym-sdk = { git = "https://github.com/nymtech/nym", branch = "develop" }

# latest stable release
nym-sdk = { git = "https://github.com/nymtech/nym", branch = "master" }
```

**No feature gates yet.** Importing `nym-sdk` pulls in everything (mixnet, tcp_proxy, client_pool, etc.) and their full dependency trees. Cargo feature flags are planned.
