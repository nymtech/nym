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

use crate::utils::file;
use reqwest::Error;

const RELATIVE_PATH: &str = "api/mixmining/topology";

pub async fn renew_periodically(validator_base_url: &str) -> Result<(), Error> {
    let url = format!("{}/{}", validator_base_url, RELATIVE_PATH);

    let topology_json = reqwest::get(&url).await?.text().await?;

    let save_path = std::env::current_exe()
        .expect("Failed to evaluate current exe path")
        .parent()
        .expect("the binary itself has no parent path?!")
        .join("public")
        .join("downloads")
        .join("topology.json");

    file::save(topology_json, save_path);
    Ok(())
}
