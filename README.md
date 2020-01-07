[![Build Status](https://travis-ci.com/nymtech/nym.svg?branch=develop)](https://travis-ci.com/nymtech/nym)

## The Nym Privacy Platform

This repository contains the full Nym platform, written in Rust.

The platform is composed of multiple Rust crates. Top-level crates include:

* client - an executable crate which you can use for interacting with Nym nodes
* mixnode - an executable mixnode crate
* sfw-provider - an executable store-and-forward provider crate. The provider acts sort of like a mailbox for mixnet messages.

### Building

#### Prerequisites

* Rust 1.39 or later. Stable works.
* The `nym` platform repo (this one).
* Checkout the [Sphinx](https://github.com/nymtech/sphinx) repo beside the `nym` repo.

Your directory structure should look like this:

```
$ tree -L 1
├── nym
│   ├── client
│   ├── mixnode
│   ├── README.md
│   └── sfw-provider
├── sphinx
```

`cargo build` will build the software.

As with any other Rust project, there are other ways to build:

* `cargo build --release` will build an optimized release version for use in production
* `cargo test` will run unit and integration tests for the crate (once)
* `cargo watch -x test` will run tests whenever you change a file in the crate. Very handy in development.
