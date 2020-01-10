## The Nym Privacy Platform

This repository contains the full Nym platform, written in Rust.

The platform is composed of multiple Rust crates. Top-level crates include:

* client - an executable crate which you can use for interacting with Nym nodes
* mixnode - an executable mixnode crate
* sfw-provider - an executable store-and-forward provider crate. The provider acts sort of like a mailbox for mixnet messages.

[![Build Status](https://travis-ci.com/nymtech/nym.svg?branch=develop)](https://travis-ci.com/nymtech/nym)

### Building

Platform build instructions are available on [on docs site](https://nymtech.net/docs/mixnet/installation/).
