#[cfg(test)]
mod tests {

    #[test]
    fn test_gateway_dto_try_from() {
        let gateway_dto = crate::db::models::GatewayDto {
            gateway_identity_key: "test_identity".to_string(),
            bonded: true,
            performance: 100,
            self_described: Some("{\"key\":\"value\"}".to_string()),
            explorer_pretty_bond: Some("{\"key\":\"value\"}".to_string()),
            last_probe_result: Some("{\"key\":\"value\"}".to_string()),
            last_probe_log: Some("log".to_string()),
            last_testrun_utc: Some(1672531200),
            last_updated_utc: 1672531200,
            moniker: "moniker".to_string(),
            security_contact: "contact".to_string(),
            details: "details".to_string(),
            website: "website".to_string(),
        };

        let http_gateway: crate::http::models::Gateway = gateway_dto.try_into().unwrap();

        assert_eq!(http_gateway.gateway_identity_key, "test_identity");
        assert!(http_gateway.bonded);
        assert_eq!(http_gateway.performance, 100);
        assert!(http_gateway.self_described.is_some());
        assert!(http_gateway.explorer_pretty_bond.is_some());
        assert!(http_gateway.last_probe_result.is_some());
        assert_eq!(http_gateway.last_probe_log, Some("log".to_string()));
        assert!(http_gateway.last_testrun_utc.is_some());
        assert!(!http_gateway.last_updated_utc.is_empty());
        assert_eq!(http_gateway.description.moniker, "moniker");
        assert_eq!(http_gateway.description.website, "website");
        assert_eq!(http_gateway.description.security_contact, "contact");
        assert_eq!(http_gateway.description.details, "details");
    }

    #[test]
    fn test_mixnode_dto_try_from() {
        let mixnode_dto = crate::db::models::MixnodeDto {
            mix_id: 1,
            bonded: true,
            is_dp_delegatee: false,
            total_stake: 1000000,
            full_details: "{\"key\":\"value\"}".to_string(),
            self_described: Some("{\"key\":\"value\"}".to_string()),
            last_updated_utc: 1672531200,
            moniker: "moniker".to_string(),
            website: "website".to_string(),
            security_contact: "contact".to_string(),
            details: "details".to_string(),
        };

        let http_mixnode: crate::http::models::Mixnode = mixnode_dto.try_into().unwrap();

        assert_eq!(http_mixnode.mix_id, 1);
        assert!(http_mixnode.bonded);
        assert!(!http_mixnode.is_dp_delegatee);
        assert_eq!(http_mixnode.total_stake, 1000000);
        assert!(http_mixnode.full_details.is_some());
        assert!(http_mixnode.self_described.is_some());
        assert!(!http_mixnode.last_updated_utc.is_empty());
        assert_eq!(http_mixnode.description.moniker, "moniker");
        assert_eq!(http_mixnode.description.website, "website");
        assert_eq!(http_mixnode.description.security_contact, "contact");
        assert_eq!(http_mixnode.description.details, "details");
    }

    #[test]
    fn test_summary_history_dto_try_from() {
        let summary_history_dto = crate::db::models::SummaryHistoryDto {
            id: 1,
            date: "2023-01-01".to_string(),
            value_json: "{\"key\":\"value\"}".to_string(),
            timestamp_utc: 1672531200,
        };

        let summary_history: crate::http::models::SummaryHistory =
            summary_history_dto.try_into().unwrap();

        assert_eq!(summary_history.date, "2023-01-01");
        assert!(summary_history.value_json.is_object());
        assert!(!summary_history.timestamp_utc.is_empty());
    }

    #[test]
    fn test_gateway_sessions_record_try_from() {
        let gateway_sessions_record = crate::db::models::GatewaySessionsRecord {
            gateway_identity_key: "test_identity".to_string(),
            node_id: 1,
            day: time::macros::date!(2023 - 01 - 01),
            unique_active_clients: 10,
            session_started: 100,
            users_hashes: Some("{\"key\":\"value\"}".to_string()),
            vpn_sessions: Some("{\"key\":\"value\"}".to_string()),
            mixnet_sessions: Some("{\"key\":\"value\"}".to_string()),
            unknown_sessions: Some("{\"key\":\"value\"}".to_string()),
        };

        let session_stats: crate::http::models::SessionStats =
            gateway_sessions_record.try_into().unwrap();

        assert_eq!(session_stats.gateway_identity_key, "test_identity");
        assert_eq!(session_stats.node_id, 1);
        assert_eq!(session_stats.day, time::macros::date!(2023 - 01 - 01));
        assert_eq!(session_stats.unique_active_clients, 10);
        assert_eq!(session_stats.session_started, 100);
        assert!(session_stats.users_hashes.is_some());
        assert!(session_stats.vpn_sessions.is_some());
        assert!(session_stats.mixnet_sessions.is_some());
        assert!(session_stats.unknown_sessions.is_some());
    }

    #[test]
    fn test_nym_node_dto_try_from() {
        let ed25519_pk = nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(&[1; 32]).unwrap();
        let x25519_pk = nym_crypto::asymmetric::x25519::PublicKey::from_bytes(&[2; 32]).unwrap();

        let nym_node_dto = crate::db::models::NymNodeDto {
            node_id: 1,
            ed25519_identity_pubkey: ed25519_pk.to_base58_string(),
            total_stake: 1000000,
            ip_addresses: serde_json::json!(["1.1.1.1"]),
            mix_port: 1789,
            x25519_sphinx_pubkey: x25519_pk.to_base58_string(),
            node_role: serde_json::json!(nym_validator_client::nym_nodes::NodeRole::Mixnode {
                layer: 1
            }),
            supported_roles: serde_json::json!(nym_validator_client::models::DeclaredRoles {
                entry: false,
                mixnode: true,
                exit_nr: false,
                exit_ipr: false,
            }),
            entry: None,
            performance: "1.0".to_string(),
            self_described: None,
            bond_info: None,
        };

        let skimmed_node: nym_validator_client::nym_api::SkimmedNode =
            nym_node_dto.try_into().unwrap();

        assert_eq!(skimmed_node.node_id, 1);
        assert_eq!(skimmed_node.ed25519_identity_pubkey, ed25519_pk);
        assert_eq!(
            skimmed_node.ip_addresses,
            vec!["1.1.1.1".parse::<std::net::IpAddr>().unwrap()]
        );
        assert_eq!(skimmed_node.mix_port, 1789);
        assert_eq!(skimmed_node.x25519_sphinx_pubkey, x25519_pk);

        match skimmed_node.role {
            nym_validator_client::nym_nodes::NodeRole::Mixnode { layer } => assert_eq!(layer, 1),
            _ => panic!("Unexpected node role"),
        }
        assert_eq!(skimmed_node.supported_roles.entry, false);
        assert_eq!(skimmed_node.supported_roles.mixnode, true);
        assert_eq!(skimmed_node.supported_roles.exit_nr, false);
        assert_eq!(skimmed_node.supported_roles.exit_ipr, false);
        assert!(skimmed_node.entry.is_none());
        assert_eq!(
            skimmed_node.performance,
            nym_contracts_common::Percent::from_percentage_value(100).unwrap()
        );
    }
}

#[test]
fn test_nym_node_insert_record_new() {
    let ed25519_pk = nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(&[1; 32]).unwrap();
    let x25519_pk = nym_crypto::asymmetric::x25519::PublicKey::from_bytes(&[2; 32]).unwrap();

    let skimmed_node = nym_validator_client::nym_api::SkimmedNode {
        node_id: 1,
        ed25519_identity_pubkey: ed25519_pk,
        ip_addresses: vec!["1.1.1.1".parse().unwrap()],
        mix_port: 1789,
        x25519_sphinx_pubkey: x25519_pk,
        role: nym_validator_client::nym_nodes::NodeRole::Mixnode { layer: 1 },
        supported_roles: nym_validator_client::models::DeclaredRoles {
            entry: false,
            mixnode: true,
            exit_nr: false,
            exit_ipr: false,
        },
        entry: None,
        performance: nym_contracts_common::Percent::from_percentage_value(100).unwrap(),
    };

    let record = crate::db::models::NymNodeInsertRecord::new(skimmed_node, None, None).unwrap();

    assert_eq!(record.node_id, 1);
    assert_eq!(
        record.ed25519_identity_pubkey,
        ed25519_pk.to_base58_string()
    );
    assert_eq!(record.total_stake, 0);
    assert_eq!(record.ip_addresses, serde_json::json!(["1.1.1.1"]));
    assert_eq!(record.mix_port, 1789);
    assert_eq!(record.x25519_sphinx_pubkey, x25519_pk.to_base58_string());
    assert_eq!(
        record.node_role,
        serde_json::json!(nym_validator_client::nym_nodes::NodeRole::Mixnode { layer: 1 })
    );
    assert_eq!(
        record.supported_roles,
        serde_json::json!(nym_validator_client::models::DeclaredRoles {
            entry: false,
            mixnode: true,
            exit_nr: false,
            exit_ipr: false,
        })
    );
    assert_eq!(record.performance, "1");
    assert!(record.entry.is_none());
    assert!(record.self_described.is_none());
    assert!(record.bond_info.is_none());
}

#[test]
fn test_nym_node_insert_record_with_entry() {
    let ed25519_pk = nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(&[1; 32]).unwrap();
    let x25519_pk = nym_crypto::asymmetric::x25519::PublicKey::from_bytes(&[2; 32]).unwrap();

    let skimmed_node = nym_validator_client::nym_api::SkimmedNode {
        node_id: 1,
        ed25519_identity_pubkey: ed25519_pk,
        ip_addresses: vec!["1.1.1.1".parse().unwrap()],
        mix_port: 1789,
        x25519_sphinx_pubkey: x25519_pk,
        role: nym_validator_client::nym_nodes::NodeRole::EntryGateway,
        supported_roles: nym_validator_client::models::DeclaredRoles {
            entry: true,
            mixnode: false,
            exit_nr: true,
            exit_ipr: false,
        },
        entry: Some(nym_validator_client::nym_nodes::BasicEntryInformation {
            hostname: Some("gateway.example.com".to_string()),
            ws_port: 9001,
            wss_port: Some(9002),
        }),
        performance: nym_contracts_common::Percent::from_percentage_value(99).unwrap(),
    };

    let record = crate::db::models::NymNodeInsertRecord::new(skimmed_node, None, None).unwrap();

    assert_eq!(record.node_id, 1);
    assert_eq!(record.total_stake, 0); // No bond info provided
    assert!(record.entry.is_some());
    assert!(record.self_described.is_none());
    assert!(record.bond_info.is_none());
    assert!(record.last_updated_utc > 0);
}

#[test]
fn test_gateway_dto_with_null_values() {
    let gateway_dto = crate::db::models::GatewayDto {
        gateway_identity_key: "test_identity".to_string(),
        bonded: false,
        performance: 0,
        self_described: None,
        explorer_pretty_bond: None,
        last_probe_result: None,
        last_probe_log: None,
        last_testrun_utc: None,
        last_updated_utc: 0,
        moniker: "".to_string(),
        security_contact: "".to_string(),
        details: "".to_string(),
        website: "".to_string(),
    };

    let http_gateway: crate::http::models::Gateway = gateway_dto.try_into().unwrap();

    assert_eq!(http_gateway.gateway_identity_key, "test_identity");
    assert!(!http_gateway.bonded);
    assert_eq!(http_gateway.performance, 0);
    assert!(http_gateway.self_described.is_none());
    assert!(http_gateway.explorer_pretty_bond.is_none());
    assert!(http_gateway.last_probe_result.is_none());
    assert!(http_gateway.last_probe_log.is_none());
    assert!(http_gateway.last_testrun_utc.is_none());
    assert_eq!(http_gateway.last_updated_utc, "1970-01-01T00:00:00Z");
}

#[test]
fn test_mixnode_dto_with_invalid_json() {
    let mixnode_dto = crate::db::models::MixnodeDto {
        mix_id: 1,
        bonded: true,
        is_dp_delegatee: false,
        total_stake: 1000000,
        full_details: "invalid json".to_string(),
        self_described: Some("also invalid".to_string()),
        last_updated_utc: 1672531200,
        moniker: "moniker".to_string(),
        website: "website".to_string(),
        security_contact: "contact".to_string(),
        details: "details".to_string(),
    };

    let http_mixnode: crate::http::models::Mixnode = mixnode_dto.try_into().unwrap();

    // Invalid JSON should result in None
    assert!(http_mixnode.full_details.is_none());
    assert_eq!(http_mixnode.self_described, Some(serde_json::Value::Null));
}

#[test]
fn test_summary_history_dto_with_invalid_json() {
    let summary_history_dto = crate::db::models::SummaryHistoryDto {
        id: 1,
        date: "2023-01-01".to_string(),
        value_json: "not valid json".to_string(),
        timestamp_utc: 1672531200,
    };

    let summary_history: crate::http::models::SummaryHistory =
        summary_history_dto.try_into().unwrap();

    assert_eq!(summary_history.date, "2023-01-01");
    // Invalid JSON should result in default (null)
    assert!(summary_history.value_json.is_null());
}

#[test]
fn test_gateway_sessions_record_with_all_none() {
    let gateway_sessions_record = crate::db::models::GatewaySessionsRecord {
        gateway_identity_key: "test_identity".to_string(),
        node_id: 1,
        day: time::macros::date!(2023 - 01 - 01),
        unique_active_clients: 0,
        session_started: 0,
        users_hashes: None,
        vpn_sessions: None,
        mixnet_sessions: None,
        unknown_sessions: None,
    };

    let session_stats: crate::http::models::SessionStats =
        gateway_sessions_record.try_into().unwrap();

    assert_eq!(session_stats.gateway_identity_key, "test_identity");
    assert_eq!(session_stats.node_id, 1);
    assert_eq!(session_stats.unique_active_clients, 0);
    assert_eq!(session_stats.session_started, 0);
    assert!(session_stats.users_hashes.is_none());
    assert!(session_stats.vpn_sessions.is_none());
    assert!(session_stats.mixnet_sessions.is_none());
    assert!(session_stats.unknown_sessions.is_none());
}

#[test]
fn test_scraper_node_info_contact_addresses() {
    use crate::db::models::{ScrapeNodeKind, ScraperNodeInfo};

    let node_info = ScraperNodeInfo {
        node_kind: ScrapeNodeKind::MixingNymNode { node_id: 123 },
        hosts: vec!["1.1.1.1".to_string(), "example.com".to_string()],
        http_api_port: 8080,
    };

    let addresses = node_info.contact_addresses();

    // Should generate multiple URLs for each host
    // Custom port (8080) should be inserted at the beginning
    assert!(addresses.contains(&"http://1.1.1.1:8080".to_string()));
    assert!(addresses.contains(&"http://example.com:8080".to_string()));
    assert!(addresses.contains(&"http://1.1.1.1:8000".to_string()));
    assert!(addresses.contains(&"https://1.1.1.1".to_string()));
    assert!(addresses.contains(&"http://example.com:8000".to_string()));
    // Check that URLs follow the expected pattern
    assert!(addresses.len() >= 8); // At least 4 URLs per host
}

#[test]
fn test_scrape_node_kind_node_id() {
    use crate::db::models::ScrapeNodeKind;

    let legacy = ScrapeNodeKind::LegacyMixnode { mix_id: 42 };
    assert_eq!(*legacy.node_id(), 42);

    let mixing = ScrapeNodeKind::MixingNymNode { node_id: 123 };
    assert_eq!(*mixing.node_id(), 123);

    let entry_exit = ScrapeNodeKind::EntryExitNymNode {
        node_id: 456,
        identity_key: "key123".to_string(),
    };
    assert_eq!(*entry_exit.node_id(), 456);
}

#[test]
fn test_nym_node_dto_with_invalid_keys() {
    let nym_node_dto = crate::db::models::NymNodeDto {
        node_id: 1,
        ed25519_identity_pubkey: "invalid_base58".to_string(),
        total_stake: 1000000,
        ip_addresses: serde_json::json!(["1.1.1.1"]),
        mix_port: 1789,
        x25519_sphinx_pubkey: "also_invalid".to_string(),
        node_role: serde_json::json!(nym_validator_client::nym_nodes::NodeRole::Mixnode {
            layer: 1
        }),
        supported_roles: serde_json::json!(nym_validator_client::models::DeclaredRoles {
            entry: false,
            mixnode: true,
            exit_nr: false,
            exit_ipr: false,
        }),
        entry: None,
        performance: "1.0".to_string(),
        self_described: None,
        bond_info: None,
    };

    let result: Result<nym_validator_client::nym_api::SkimmedNode, _> = nym_node_dto.try_into();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("ed25519_identity_pubkey"));
}

#[test]
fn test_nym_node_dto_with_invalid_performance() {
    let ed25519_pk = nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(&[1; 32]).unwrap();
    let x25519_pk = nym_crypto::asymmetric::x25519::PublicKey::from_bytes(&[2; 32]).unwrap();

    let nym_node_dto = crate::db::models::NymNodeDto {
        node_id: 1,
        ed25519_identity_pubkey: ed25519_pk.to_base58_string(),
        total_stake: 1000000,
        ip_addresses: serde_json::json!(["1.1.1.1"]),
        mix_port: 1789,
        x25519_sphinx_pubkey: x25519_pk.to_base58_string(),
        node_role: serde_json::json!(nym_validator_client::nym_nodes::NodeRole::Mixnode {
            layer: 1
        }),
        supported_roles: serde_json::json!(nym_validator_client::models::DeclaredRoles {
            entry: false,
            mixnode: true,
            exit_nr: false,
            exit_ipr: false,
        }),
        entry: None,
        performance: "invalid_percent".to_string(),
        self_described: None,
        bond_info: None,
    };

    let result: Result<nym_validator_client::nym_api::SkimmedNode, _> = nym_node_dto.try_into();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("can't parse Percent"));
}
