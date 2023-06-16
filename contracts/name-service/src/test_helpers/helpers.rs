use cosmwasm_std::{Coin, Event, Response};
use cw_multi_test::AppResponse;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

pub fn nyms(amount: u64) -> Coin {
    Coin::new(amount.into(), "unym")
}

pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    ChaCha20Rng::from_seed(dummy_seed)
}

pub fn get_event_types(response: &Response, event_type: &str) -> Vec<Event> {
    response
        .events
        .iter()
        .filter(|ev| ev.ty == event_type)
        .cloned()
        .collect()
}

pub fn get_attribute(response: &Response, event_type: &str, key: &str) -> String {
    get_event_types(response, event_type)
        .first()
        .unwrap()
        .attributes
        .iter()
        .find(|attr| attr.key == key)
        .unwrap()
        .value
        .clone()
}

pub fn get_app_event_types(response: &AppResponse, event_type: &str) -> Vec<Event> {
    response
        .events
        .iter()
        .filter(|ev| ev.ty == event_type)
        .cloned()
        .collect()
}

pub fn get_app_attribute(response: &AppResponse, event_type: &str, key: &str) -> String {
    get_app_event_types(response, event_type)
        .first()
        .unwrap()
        .attributes
        .iter()
        .find(|attr| attr.key == key)
        .unwrap()
        .value
        .clone()
}
