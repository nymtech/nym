use mixnet_contract::{HumanAddr, PagedGatewayResponse, PagedResponse};
use reqwest::Method;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

pub struct Config {
    rpc_server_base_url: String,
    mixnet_contract_address: String,
    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
}

impl Config {
    pub fn new<S: Into<String>>(base_url: S, mixnet_contract_address: S) -> Self {
        Config {
            rpc_server_base_url: base_url.into(),
            mixnet_contract_address: mixnet_contract_address.into(),
            mixnode_page_limit: None,
            gateway_page_limit: None,
        }
    }

    pub fn with_mixnode_page_limit(mut self, limit: Option<u32>) -> Config {
        self.mixnode_page_limit = limit;
        self
    }

    pub fn with_gateway_page_limit(mut self, limit: Option<u32>) -> Config {
        self.gateway_page_limit = limit;
        self
    }
}

pub struct Client {
    config: Config,
    reqwest_client: reqwest::Client,
}

impl Client {
    pub fn new(config: Config) -> Self {
        let reqwest_client = reqwest::Client::new();
        Client {
            config,
            reqwest_client,
        }
    }

    fn base_query_path(&self) -> String {
        format!(
            "{}/wasm/contract/{}/smart",
            self.config.rpc_server_base_url, self.config.mixnet_contract_address
        )
    }

    async fn get_mix_nodes_paged(&self, start_after: Option<HumanAddr>) {
        let query_content_json = serde_json::to_string(&QueryRequest::GetMixNodes {
            limit: self.config.mixnode_page_limit,
            start_after,
        })
        .expect("serde was incorrectly implemented on QueryRequest::GetMixNodes!");

        println!("req json: {}", query_content_json);

        let query_content = base64::encode(query_content_json);

        let query_url = format!(
            "{}/{}?encoding=base64",
            self.base_query_path(),
            query_content
        );

        let res = self.reqwest_client.get(query_url).send().await;

        println!("{:?}", res);
        let a: SmartQueryResult = res.unwrap().json().await.unwrap();
        // let a = res.unwrap().text().await.unwrap();
        // let foo: SmartQueryResult = serde_json::from_str(&a).unwrap();
        println!("got {:?}", a)
        // let mut req_builder = self.reqwest_client.request(Method::GET)
    }

    pub async fn get_mix_nodes(&self) {}

    async fn get_gateways_paged(&self) {}

    pub async fn get_gateways(&self) {}
}

// TODO: this is really a duplicate code but it really does not feel
// like it belongs in the common crate because it's TOO contract specific...
// I'm not entirely sure what to do about it now.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum QueryRequest {
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<HumanAddr>,
    },
    GetGateways {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum QueryResponse {
    MixNodes(PagedResponse),
    Gateways(PagedGatewayResponse),
}

#[derive(Deserialize, Debug)]
struct SmartResult {
    #[serde(deserialize_with = "de_query_response_from_str")]
    smart: QueryResponse,
}

#[derive(Deserialize, Debug)]
struct SmartQueryResult {
    #[serde(deserialize_with = "de_i64_from_str")]
    height: i64,
    result: SmartResult,
}

fn de_query_response_from_str<'de, D>(deserializer: D) -> Result<QueryResponse, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let b64_decoded = base64::decode(&s).map_err(serde::de::Error::custom)?;

    let json_string = String::from_utf8(b64_decoded).map_err(serde::de::Error::custom)?;
    serde_json::from_str(&json_string).map_err(serde::de::Error::custom)
}

fn de_i64_from_str<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    i64::from_str(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn foo() {
        let base_url = "http://localhost:1317";
        let contract = "nym10pyejy66429refv3g35g2t7am0was7ya69su6d";

        let client = Client::new(Config::new(base_url, contract));

        client.get_mix_nodes_paged(None).await;
    }
}
