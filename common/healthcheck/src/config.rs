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
