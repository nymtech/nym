// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Pins the JSON of every ecash message to what the sylvia-generated types produced.

use crate::contract::sv;
use cosmwasm_std::{coin, from_json, to_json_string};
use nym_ecash_contract_common::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, WhitelistedDeposit,
};

struct WireCase<C, S> {
    json: &'static str,
    common: C,
    sylvia: S,
}

fn execute_cases() -> Vec<WireCase<ExecuteMsg, sv::ExecMsg>> {
    vec![
        WireCase {
            json: r#"{"deposit_ticket_book_funds":{"identity_key":"GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK"}}"#,
            common: ExecuteMsg::DepositTicketBookFunds {
                identity_key: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK".into(),
            },
            sylvia: sv::ExecMsg::DepositTicketBookFunds {
                identity_key: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK".into(),
            },
        },
        WireCase {
            json: r#"{"request_redemption":{"commitment_bs58":"n1commitment","number_of_tickets":100}}"#,
            common: ExecuteMsg::RequestRedemption {
                commitment_bs58: "n1commitment".into(),
                number_of_tickets: 100,
            },
            sylvia: sv::ExecMsg::RequestRedemption {
                commitment_bs58: "n1commitment".into(),
                number_of_tickets: 100,
            },
        },
        WireCase {
            json: r#"{"redeem_tickets":{"n":100,"gw":"n1gateway"}}"#,
            common: ExecuteMsg::RedeemTickets {
                n: 100,
                gw: "n1gateway".into(),
            },
            sylvia: sv::ExecMsg::RedeemTickets {
                n: 100,
                gw: "n1gateway".into(),
            },
        },
        WireCase {
            json: r#"{"update_admin":{"admin":"n1admin"}}"#,
            common: ExecuteMsg::UpdateAdmin {
                admin: "n1admin".into(),
            },
            sylvia: sv::ExecMsg::UpdateAdmin {
                admin: "n1admin".into(),
            },
        },
        WireCase {
            json: r#"{"update_default_deposit_value":{"new_deposit":{"denom":"unym","amount":"75000000"}}}"#,
            common: ExecuteMsg::UpdateDefaultDepositValue {
                new_deposit: coin(75000000, "unym"),
            },
            sylvia: sv::ExecMsg::UpdateDefaultDepositValue {
                new_deposit: coin(75000000, "unym"),
            },
        },
        WireCase {
            json: r#"{"set_reduced_deposit_price":{"address":"n1address","deposit":{"denom":"unym","amount":"50000000"}}}"#,
            common: ExecuteMsg::SetReducedDepositPrice {
                address: "n1address".into(),
                deposit: coin(50000000, "unym"),
            },
            sylvia: sv::ExecMsg::SetReducedDepositPrice {
                address: "n1address".into(),
                deposit: coin(50000000, "unym"),
            },
        },
        WireCase {
            json: r#"{"remove_reduced_deposit_price":{"address":"n1address"}}"#,
            common: ExecuteMsg::RemoveReducedDepositPrice {
                address: "n1address".into(),
            },
            sylvia: sv::ExecMsg::RemoveReducedDepositPrice {
                address: "n1address".into(),
            },
        },
        WireCase {
            json: r#"{"propose_to_blacklist":{"public_key":"n1publickey"}}"#,
            common: ExecuteMsg::ProposeToBlacklist {
                public_key: "n1publickey".into(),
            },
            sylvia: sv::ExecMsg::ProposeToBlacklist {
                public_key: "n1publickey".into(),
            },
        },
        WireCase {
            json: r#"{"add_to_blacklist":{"public_key":"n1publickey"}}"#,
            common: ExecuteMsg::AddToBlacklist {
                public_key: "n1publickey".into(),
            },
            sylvia: sv::ExecMsg::AddToBlacklist {
                public_key: "n1publickey".into(),
            },
        },
    ]
}

fn query_cases() -> Vec<WireCase<QueryMsg, sv::QueryMsg>> {
    vec![
        WireCase {
            json: r#"{"get_blacklist_paged":{"limit":10,"start_after":"n1publickey"}}"#,
            common: QueryMsg::GetBlacklistPaged {
                limit: Some(10),
                start_after: Some("n1publickey".into()),
            },
            sylvia: sv::QueryMsg::GetBlacklistPaged {
                limit: Some(10),
                start_after: Some("n1publickey".into()),
            },
        },
        WireCase {
            json: r#"{"get_blacklist_paged":{"limit":null,"start_after":null}}"#,
            common: QueryMsg::GetBlacklistPaged {
                limit: None,
                start_after: None,
            },
            sylvia: sv::QueryMsg::GetBlacklistPaged {
                limit: None,
                start_after: None,
            },
        },
        WireCase {
            json: r#"{"get_blacklisted_account":{"public_key":"n1publickey"}}"#,
            common: QueryMsg::GetBlacklistedAccount {
                public_key: "n1publickey".into(),
            },
            sylvia: sv::QueryMsg::GetBlacklistedAccount {
                public_key: "n1publickey".into(),
            },
        },
        WireCase {
            json: r#"{"get_default_deposit_amount":{}}"#,
            common: QueryMsg::GetDefaultDepositAmount {},
            sylvia: sv::QueryMsg::GetDefaultDepositAmount {},
        },
        WireCase {
            json: r#"{"get_reduced_deposit_amount":{"address":"n1address"}}"#,
            common: QueryMsg::GetReducedDepositAmount {
                address: "n1address".into(),
            },
            sylvia: sv::QueryMsg::GetReducedDepositAmount {
                address: "n1address".into(),
            },
        },
        WireCase {
            json: r#"{"get_all_whitelisted_accounts":{}}"#,
            common: QueryMsg::GetAllWhitelistedAccounts {},
            sylvia: sv::QueryMsg::GetAllWhitelistedAccounts {},
        },
        WireCase {
            json: r#"{"get_deposit":{"deposit_id":42}}"#,
            common: QueryMsg::GetDeposit { deposit_id: 42 },
            sylvia: sv::QueryMsg::GetDeposit { deposit_id: 42 },
        },
        WireCase {
            json: r#"{"get_latest_deposit":{}}"#,
            common: QueryMsg::GetLatestDeposit {},
            sylvia: sv::QueryMsg::GetLatestDeposit {},
        },
        WireCase {
            json: r#"{"get_deposits_paged":{"limit":10,"start_after":5}}"#,
            common: QueryMsg::GetDepositsPaged {
                limit: Some(10),
                start_after: Some(5),
            },
            sylvia: sv::QueryMsg::GetDepositsPaged {
                limit: Some(10),
                start_after: Some(5),
            },
        },
        WireCase {
            json: r#"{"get_deposits_paged":{"limit":null,"start_after":null}}"#,
            common: QueryMsg::GetDepositsPaged {
                limit: None,
                start_after: None,
            },
            sylvia: sv::QueryMsg::GetDepositsPaged {
                limit: None,
                start_after: None,
            },
        },
        WireCase {
            json: r#"{"get_deposits_statistics":{}}"#,
            common: QueryMsg::GetDepositsStatistics {},
            sylvia: sv::QueryMsg::GetDepositsStatistics {},
        },
    ]
}

fn instantiate_cases() -> Vec<WireCase<InstantiateMsg, sv::InstantiateMsg>> {
    vec![WireCase {
        json: r#"{"holding_account":"n1holding","multisig_addr":"n1multisig","group_addr":"n1group","deposit_amount":{"denom":"unym","amount":"75000000"}}"#,
        common: InstantiateMsg {
            holding_account: "n1holding".into(),
            multisig_addr: "n1multisig".into(),
            group_addr: "n1group".into(),
            deposit_amount: coin(75000000, "unym"),
        },
        sylvia: sv::InstantiateMsg {
            holding_account: "n1holding".into(),
            multisig_addr: "n1multisig".into(),
            group_addr: "n1group".into(),
            deposit_amount: coin(75000000, "unym"),
        },
    }]
}

fn migrate_cases() -> Vec<WireCase<MigrateMsg, sv::MigrateMsg>> {
    let whitelisted = || WhitelistedDeposit {
        address: "n1address".into(),
        deposit: coin(50000000, "unym"),
    };

    vec![
        WireCase {
            json: r#"{"initial_whitelist":[]}"#,
            common: MigrateMsg {
                initial_whitelist: vec![],
            },
            sylvia: sv::MigrateMsg {
                initial_whitelist: vec![],
            },
        },
        WireCase {
            json: r#"{"initial_whitelist":[{"address":"n1address","deposit":{"denom":"unym","amount":"50000000"}}]}"#,
            common: MigrateMsg {
                initial_whitelist: vec![whitelisted()],
            },
            sylvia: sv::MigrateMsg {
                initial_whitelist: vec![whitelisted()],
            },
        },
    ]
}

#[test]
fn execute_messages_match_sylvia_wire_format() {
    for case in execute_cases() {
        assert_eq!(to_json_string(&case.common).unwrap(), case.json);
        assert_eq!(to_json_string(&case.sylvia).unwrap(), case.json);
        assert_eq!(from_json::<ExecuteMsg>(case.json).unwrap(), case.common);
        assert_eq!(
            from_json::<sv::ContractExecMsg>(case.json).unwrap(),
            sv::ContractExecMsg::NymEcashContract(case.sylvia)
        );
    }
}

#[test]
fn query_messages_match_sylvia_wire_format() {
    for case in query_cases() {
        assert_eq!(to_json_string(&case.common).unwrap(), case.json);
        assert_eq!(to_json_string(&case.sylvia).unwrap(), case.json);
        assert_eq!(from_json::<QueryMsg>(case.json).unwrap(), case.common);
        assert_eq!(
            from_json::<sv::ContractQueryMsg>(case.json).unwrap(),
            sv::ContractQueryMsg::NymEcashContract(case.sylvia)
        );
    }
}

#[test]
fn instantiate_message_matches_sylvia_wire_format() {
    for case in instantiate_cases() {
        assert_eq!(to_json_string(&case.common).unwrap(), case.json);
        assert_eq!(to_json_string(&case.sylvia).unwrap(), case.json);
        assert_eq!(from_json::<InstantiateMsg>(case.json).unwrap(), case.common);
        assert_eq!(
            from_json::<sv::InstantiateMsg>(case.json).unwrap(),
            case.sylvia
        );
    }
}

#[test]
fn migrate_messages_match_sylvia_wire_format() {
    for case in migrate_cases() {
        assert_eq!(to_json_string(&case.common).unwrap(), case.json);
        assert_eq!(to_json_string(&case.sylvia).unwrap(), case.json);
        assert_eq!(from_json::<MigrateMsg>(case.json).unwrap(), case.common);
        assert_eq!(from_json::<sv::MigrateMsg>(case.json).unwrap(), case.sylvia);
    }
}

// sylvia had a separate `get_required_deposit_amount` query; the common crate
// declares that name as an alias of `GetDefaultDepositAmount`
#[test]
fn sylvia_required_deposit_query_parses_as_default_deposit_query() {
    let json = r#"{"get_required_deposit_amount":{}}"#;

    assert_eq!(
        to_json_string(&sv::QueryMsg::GetRequiredDepositAmount {}).unwrap(),
        json
    );
    assert_eq!(
        from_json::<QueryMsg>(json).unwrap(),
        QueryMsg::GetDefaultDepositAmount {}
    );
}
