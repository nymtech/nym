use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(rename(deserialize = "healthcheck"))]
    pub health_check: HealthCheck,
}

#[derive(Deserialize, Debug)]
pub struct HealthCheck {
    #[serde(rename(deserialize = "directory-server"))]
    pub directory_server: String,
}
