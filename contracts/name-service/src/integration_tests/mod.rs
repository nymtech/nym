//! Integration tests using cw-multi-test.

mod delete;
mod name_id;
mod query;
mod register;
mod test_name;
mod test_setup;

#[test]
fn instantiate_contract() {
    test_setup::TestSetup::new();
}
