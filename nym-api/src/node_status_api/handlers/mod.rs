// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::v2::AxumAppState;
use axum::Router;

pub(crate) mod network_monitor;
pub(crate) mod unstable;
pub(crate) mod without_monitor;

pub(crate) fn node_status_routes(network_monitor: bool) -> Router<AxumAppState> {
    // in the minimal variant we would not have access to endpoints relying on existence
    // of the network monitor and the associated storage
    let without_network_monitor = without_monitor::mandatory_routes();

    if network_monitor {
        let with_network_monitor = network_monitor::network_monitor_routes();

        with_network_monitor.merge(without_network_monitor)
    } else {
        without_network_monitor
    }
}
