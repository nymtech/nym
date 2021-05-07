// Copyright 2021 Nym Technologies SA
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

use crypto::asymmetric::identity;
use std::sync::Arc;

pub(crate) mod listener;
pub(crate) mod packet;
pub(crate) mod sender;

pub struct LatencyMeasurer {
    identity: Arc<identity::KeyPair>,
    batch_size: usize,
    packets_per_node: usize,
}

impl LatencyMeasurer {
    async fn start_listening() {}

    async fn send() {}

    pub async fn run(&self) {
        //
    }
}
