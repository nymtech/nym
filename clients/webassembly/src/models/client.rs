use directory_client::DirectoryClient;
use std::convert::TryInto;
use topology::NymTopology;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
pub struct ClientTest {
    version: String,
    directory_server: String,
}

#[wasm_bindgen]
impl ClientTest {
    // #[wasm_bindgen(constructor)]
    // pub fn new() -> Self {
    //     ClientTest {
    //         version: "0.8".to_string(),
    //         directory_server: "http://localhost:8080".to_string(),
    //     }
    // }
    //
    pub async fn do_foomp() -> String {
        let topology = Self::get_topology().await;
        format!("{:#?}", topology)

        // "aa".to_string()
        // spawn_local(async move { loop {} })
    }

    async fn get_topology() -> NymTopology {
        let dir = "http://localhost:8080".to_string();
        let ver = "0.8".to_string();

        let directory_client_config = directory_client::Config::new(dir);
        let directory_client = directory_client::Client::new(directory_client_config);

        match directory_client.get_topology().await {
            Err(err) => panic!(err),
            Ok(topology) => {
                let nym_topology: NymTopology =
                    topology.try_into().ok().expect("todo error handling etc");
                nym_topology.filter_system_version(&ver)
            }
        }
    }
}
