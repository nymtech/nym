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

pub mod health_check_get;
pub mod metrics_mixes_get;
pub mod metrics_mixes_post;
pub mod mix_mining_batch_status_post;
pub mod mix_mining_status_post;
pub mod presence_coconodes_post;
pub mod presence_gateways_post;
pub mod presence_mixnodes_post;
pub mod presence_providers_post;
pub mod presence_topology_get;

use serde::{de::DeserializeOwned, Serialize};

pub(crate) trait DirectoryRequest {
    fn url(&self) -> String;
}

pub(crate) trait DirectoryGetRequest: DirectoryRequest {
    // perhaps the name of this is not the best because it's technically not a JSON,
    // but something that can be deserialised from JSON.
    // I'm open to all suggestions on how to rename it
    type JSONResponse: DeserializeOwned;

    fn new(base_url: &str) -> Self;
}

pub(crate) trait DirectoryPostRequest: DirectoryRequest {
    // Similarly this, it's something that can be serialized into a JSON
    type Payload: Serialize + ?Sized;

    fn new(base_url: &str, payload: Self::Payload) -> Self;
    fn json_payload(&self) -> &Self::Payload;
}
