// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::config::Config;
// use crate::config::{Config, SocketType};

pub mod init;
pub mod run;

pub(crate) fn override_config(mut config: Config, host: String) -> Config {
    // config = config.with_listening_port(1783);
    if !host.is_empty() {
        config = config.with_listening_host(host);
        config = config.announce_host_from_listening_host()
    }
    // config = config.with_socket(SocketType::WebSocket);
    config
}
