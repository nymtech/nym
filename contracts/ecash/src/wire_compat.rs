// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Pins the JSON of every ecash message to what the sylvia-generated types produced.

use cosmwasm_std::{coin, from_json, to_json_string};
use nym_ecash_contract_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

struct WireCase<M> {
    json: &'static str,
    msg: M,
}

fn execute_cases() -> Vec<WireCase<ExecuteMsg>> {
    vec![
        WireCase {
            json: r#"{"deposit_ticket_book_funds":{"identity_key":"GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK"}}"#,
            msg: ExecuteMsg::DepositTicketBookFunds {
                identity_key: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK".into(),
            },
        },
        WireCase {
            json: r#"{"request_redemption":{"commitment_bs58":"n1commitment","number_of_tickets":100}}"#,
            msg: ExecuteMsg::RequestRedemption {
                commitment_bs58: "n1commitment".into(),
                number_of_tickets: 100,
            },
        },
        WireCase {
            json: r#"{"redeem_tickets":{"n":100,"gw":"n1gateway"}}"#,
            msg: ExecuteMsg::RedeemTickets {
                n: 100,
                gw: "n1gateway".into(),
            },
        },
        WireCase {
            json: r#"{"update_admin":{"admin":"n1admin"}}"#,
            msg: ExecuteMsg::UpdateAdmin {
                admin: "n1admin".into(),
            },
        },
        WireCase {
            json: r#"{"update_default_deposit_value":{"new_deposit":{"denom":"unym","amount":"75000000"}}}"#,
            msg: ExecuteMsg::UpdateDefaultDepositValue {
                new_deposit: coin(75000000, "unym"),
            },
        },
        WireCase {
            json: r#"{"set_reduced_deposit_price":{"address":"n1address","deposit":{"denom":"unym","amount":"50000000"}}}"#,
            msg: ExecuteMsg::SetReducedDepositPrice {
                address: "n1address".into(),
                deposit: coin(50000000, "unym"),
            },
        },
        WireCase {
            json: r#"{"remove_reduced_deposit_price":{"address":"n1address"}}"#,
            msg: ExecuteMsg::RemoveReducedDepositPrice {
                address: "n1address".into(),
            },
        },
        WireCase {
            json: r#"{"propose_to_blacklist":{"public_key":"n1publickey"}}"#,
            msg: ExecuteMsg::ProposeToBlacklist {
                public_key: "n1publickey".into(),
            },
        },
        WireCase {
            json: r#"{"add_to_blacklist":{"public_key":"n1publickey"}}"#,
            msg: ExecuteMsg::AddToBlacklist {
                public_key: "n1publickey".into(),
            },
        },
    ]
}

fn query_cases() -> Vec<WireCase<QueryMsg>> {
    vec![
        WireCase {
            json: r#"{"get_blacklist_paged":{"limit":10,"start_after":"n1publickey"}}"#,
            msg: QueryMsg::GetBlacklistPaged {
                limit: Some(10),
                start_after: Some("n1publickey".into()),
            },
        },
        WireCase {
            json: r#"{"get_blacklist_paged":{"limit":null,"start_after":null}}"#,
            msg: QueryMsg::GetBlacklistPaged {
                limit: None,
                start_after: None,
            },
        },
        WireCase {
            json: r#"{"get_blacklisted_account":{"public_key":"n1publickey"}}"#,
            msg: QueryMsg::GetBlacklistedAccount {
                public_key: "n1publickey".into(),
            },
        },
        WireCase {
            json: r#"{"get_default_deposit_amount":{}}"#,
            msg: QueryMsg::GetDefaultDepositAmount {},
        },
        WireCase {
            json: r#"{"get_reduced_deposit_amount":{"address":"n1address"}}"#,
            msg: QueryMsg::GetReducedDepositAmount {
                address: "n1address".into(),
            },
        },
        WireCase {
            json: r#"{"get_all_whitelisted_accounts":{}}"#,
            msg: QueryMsg::GetAllWhitelistedAccounts {},
        },
        WireCase {
            json: r#"{"get_deposit":{"deposit_id":42}}"#,
            msg: QueryMsg::GetDeposit { deposit_id: 42 },
        },
        WireCase {
            json: r#"{"get_latest_deposit":{}}"#,
            msg: QueryMsg::GetLatestDeposit {},
        },
        WireCase {
            json: r#"{"get_deposits_paged":{"limit":10,"start_after":5}}"#,
            msg: QueryMsg::GetDepositsPaged {
                limit: Some(10),
                start_after: Some(5),
            },
        },
        WireCase {
            json: r#"{"get_deposits_paged":{"limit":null,"start_after":null}}"#,
            msg: QueryMsg::GetDepositsPaged {
                limit: None,
                start_after: None,
            },
        },
        WireCase {
            json: r#"{"get_deposits_statistics":{}}"#,
            msg: QueryMsg::GetDepositsStatistics {},
        },
    ]
}

fn instantiate_cases() -> Vec<WireCase<InstantiateMsg>> {
    vec![WireCase {
        json: r#"{"holding_account":"n1holding","multisig_addr":"n1multisig","group_addr":"n1group","deposit_amount":{"denom":"unym","amount":"75000000"}}"#,
        msg: InstantiateMsg {
            holding_account: "n1holding".into(),
            multisig_addr: "n1multisig".into(),
            group_addr: "n1group".into(),
            deposit_amount: coin(75000000, "unym"),
        },
    }]
}

#[test]
fn execute_messages_match_sylvia_wire_format() {
    for case in execute_cases() {
        assert_eq!(to_json_string(&case.msg).unwrap(), case.json);
        assert_eq!(from_json::<ExecuteMsg>(case.json).unwrap(), case.msg);
    }
}

#[test]
fn query_messages_match_sylvia_wire_format() {
    for case in query_cases() {
        assert_eq!(to_json_string(&case.msg).unwrap(), case.json);
        assert_eq!(from_json::<QueryMsg>(case.json).unwrap(), case.msg);
    }
}

#[test]
fn instantiate_message_matches_sylvia_wire_format() {
    for case in instantiate_cases() {
        assert_eq!(to_json_string(&case.msg).unwrap(), case.json);
        assert_eq!(from_json::<InstantiateMsg>(case.json).unwrap(), case.msg);
    }
}

#[test]
fn migrate_message_is_empty() {
    assert_eq!(to_json_string(&MigrateMsg {}).unwrap(), "{}");
    assert_eq!(from_json::<MigrateMsg>("{}").unwrap(), MigrateMsg {});

    // the tiered-pricing migration's field is accepted and ignored
    let leftover = r#"{"initial_whitelist":[{"address":"n1address","deposit":{"denom":"unym","amount":"50000000"}}]}"#;
    assert_eq!(from_json::<MigrateMsg>(leftover).unwrap(), MigrateMsg {});
}

// sylvia had a separate `get_required_deposit_amount` query and accepted
// neither `update_deposit_value` nor `GetRequiredDepositAmount`; the common
// crate declares all three as aliases
#[test]
fn aliases_parse_as_their_variants() {
    assert_eq!(
        from_json::<QueryMsg>(r#"{"get_required_deposit_amount":{}}"#).unwrap(),
        QueryMsg::GetDefaultDepositAmount {}
    );
    assert_eq!(
        from_json::<QueryMsg>(r#"{"GetRequiredDepositAmount":{}}"#).unwrap(),
        QueryMsg::GetDefaultDepositAmount {}
    );
    assert_eq!(
        from_json::<ExecuteMsg>(
            r#"{"update_deposit_value":{"new_deposit":{"denom":"unym","amount":"75000000"}}}"#
        )
        .unwrap(),
        ExecuteMsg::UpdateDefaultDepositValue {
            new_deposit: coin(75000000, "unym"),
        }
    );
}

#[test]
fn unknown_fields_are_ignored() {
    let json = r#"{"deposit_ticket_book_funds":{"identity_key":"GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK","unexpected":1}}"#;
    assert_eq!(
        from_json::<ExecuteMsg>(json).unwrap(),
        ExecuteMsg::DepositTicketBookFunds {
            identity_key: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK".into(),
        }
    );
}

#[test]
fn unknown_variant_is_rejected() {
    assert!(from_json::<ExecuteMsg>(r#"{"no_such_message":{}}"#).is_err());
}
