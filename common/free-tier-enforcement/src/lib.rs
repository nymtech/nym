// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Free-tier datapath enforcement for `nym-node` (Linux-only): the shared `tc`
//! rate-limit pool and the `iptables` walled-garden managers. `nym-node` wires
//! these in; the network-namespace integration tests under `tests/` exercise the
//! datapath against a real kernel. The managers themselves land with tasks 4 and 5.
