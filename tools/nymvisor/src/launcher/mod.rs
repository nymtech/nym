// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;

pub(crate) struct DaemonLauncher {
    config: Config,
}

impl DaemonLauncher {
    pub(crate) fn new(config: Config) -> Self {
        todo!()
    }

    // responsible for running until exit or until update is detected
    pub(crate) fn run(&self, args: Vec<String>) {
        /*

           tokio select on:
           - daemon terminating
           - upgrade-plan.json changes
           - https://nymtech.net/.wellknown/<DAEMON_NAME>/update-info.json changes
           
           
           // todo: maybe move to a higher layer
           - signals received (to propagate them to daemon before terminating to prevent creating zombie processes)

        */

        todo!()
    }
}
