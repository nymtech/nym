use cosmwasm_std::{Coin, Response};
use cw_multi_test::AppResponse;

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
