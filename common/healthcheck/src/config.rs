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

use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct HealthCheck {
    #[serde(rename(deserialize = "directory-server"))]
    pub directory_server: String,

    pub interval: f64, // in seconds

    #[serde(rename(deserialize = "resolution-timeout"))]
    pub resolution_timeout: f64, // in seconds

    #[serde(rename(deserialize = "test-packets-per-node"))]
    pub num_test_packets: usize,
}
