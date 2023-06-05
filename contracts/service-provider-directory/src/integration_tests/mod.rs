//! Integration tests using cw-multi-test.

mod announce;
mod delete;
mod query;
mod service_id;
mod test_service;
mod test_setup;

#[test]
fn instantiate_contract() {
    test_setup::TestSetup::new();
}
