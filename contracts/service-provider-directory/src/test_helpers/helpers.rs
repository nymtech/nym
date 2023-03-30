use cosmwasm_std::{Coin, Response};
use cw_multi_test::AppResponse;
use nym_service_provider_directory_common::Service;

use crate::msg::ExecuteMsg;

pub fn nyms(amount: u64) -> Coin {
    Coin::new(amount.into(), "unym")
}

pub fn get_attribute(res: Response, key: &str) -> String {
    res.attributes
        .iter()
        .find(|attr| attr.key == key)
        .unwrap()
        .value
        .clone()
}

pub fn get_app_attribute(response: &AppResponse, key: &str) -> String {
    let wasm = response.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    wasm.attributes
        .iter()
        .find(|attr| attr.key == key)
        .unwrap()
        .value
        .clone()
}

impl From<Service> for ExecuteMsg {
    fn from(service: Service) -> Self {
        ExecuteMsg::Announce {
            nym_address: service.nym_address,
            service_type: service.service_type,
        }
    }
}
