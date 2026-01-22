use crate::http::models::Gateway;

use super::*;

#[test]
fn test_weighted_score_calculation() {
    // Helper function to create a test gateway
    fn create_test_gateway(performance: u8) -> Gateway {
        Gateway {
            gateway_identity_key: "test_key".to_string(),
            bonded: true,
            performance,
            self_described: None,
            explorer_pretty_bond: None,
            description: nym_node_requests::api::v1::node::models::NodeDescription {
                moniker: "test".to_string(),
                details: "test".to_string(),
                security_contact: "test@example.com".to_string(),
                website: "https://example.com".to_string(),
            },
            last_probe_result: None,
            last_probe_log: None,
            last_testrun_utc: None,
            last_updated_utc: "2025-10-10T00:00:00Z".to_string(),
            routing_score: 0.0,
            config_score: 0,
            bridges: None,
        }
    }

    // Helper function to create a test probe outcome
    fn create_test_probe_outcome(
        download_speed_mbps: f64,
        ping_ips_performance: f32,
    ) -> LastProbeResult {
        let duration_sec = 1.0;
        let file_size_mb = download_speed_mbps;

        LastProbeResult {
            node: "test_node".to_string(),
            used_entry: "test_entry".to_string(),
            outcome: ProbeOutcome {
                as_entry: Entry::Tested(EntryTestResult {
                    can_connect: true,
                    can_route: true,
                }),
                as_exit: None,
                wg: Some(WgProbeResults {
                    can_register: true,
                    can_handshake: Some(true),
                    can_resolve_dns: Some(true),
                    ping_hosts_performance: Some(ping_ips_performance),
                    ping_ips_performance: Some(ping_ips_performance),
                    can_query_metadata_v4: Some(true),
                    can_handshake_v4: true,
                    can_resolve_dns_v4: true,
                    ping_hosts_performance_v4: ping_ips_performance,
                    ping_ips_performance_v4: ping_ips_performance,
                    can_handshake_v6: true,
                    can_resolve_dns_v6: true,
                    ping_hosts_performance_v6: ping_ips_performance,
                    ping_ips_performance_v6: ping_ips_performance,
                    download_duration_sec_v4: (duration_sec * 1000.0) as u64,
                    download_duration_milliseconds_v4: Some((duration_sec * 1000.0) as u64),
                    downloaded_file_size_bytes_v4: Some((file_size_mb * 1024.0 * 1024.0) as u64),
                    downloaded_file_v4: "test".to_string(),
                    download_error_v4: "".to_string(),
                    download_duration_sec_v6: 0,
                    download_duration_milliseconds_v6: Some(0),
                    downloaded_file_size_bytes_v6: Some(0),
                    downloaded_file_v6: "".to_string(),
                    download_error_v6: "".to_string(),
                }),
            },
        }
    }

    // Test case 1: Excellent node (should be High)
    let gateway = create_test_gateway(90); // 90% mixnet performance
    let probe = create_test_probe_outcome(6.0, 1.0); // 6 Mbps, 100% ping
    let score = calculate_score(&gateway, &probe);
    assert_eq!(score, ScoreValue::High, "Excellent node should be High");

    // Test case 2: Good node (should be High with weighted scoring)
    let gateway = create_test_gateway(90); // 90% mixnet performance
    let probe = create_test_probe_outcome(3.0, 0.9); // 3 Mbps (0.75 score), 90% ping
    let score = calculate_score(&gateway, &probe);
    assert_eq!(
        score,
        ScoreValue::High,
        "Good node should be High with weighted scoring"
    );

    // Test case 3: Medium node
    let gateway = create_test_gateway(80); // 80% mixnet performance
    let probe = create_test_probe_outcome(1.5, 0.8); // 1.5 Mbps (0.5 score), 80% ping
    let score = calculate_score(&gateway, &probe);
    assert_eq!(score, ScoreValue::Medium, "Medium node should be Medium");

    // Test case 4: Poor node
    let gateway = create_test_gateway(60); // 60% mixnet performance
    let probe = create_test_probe_outcome(0.3, 0.3); // 0.3 Mbps (0.1 score), 30% ping
    let score = calculate_score(&gateway, &probe);
    assert_eq!(score, ScoreValue::Low, "Poor node should be Low");

    // Test case 5: Failed node
    let gateway = create_test_gateway(10); // 10% mixnet performance
    let probe = create_test_probe_outcome(0.1, 0.0); // 0.1 Mbps (0.1 score), 0% ping
    let score = calculate_score(&gateway, &probe);
    assert_eq!(score, ScoreValue::Offline, "Failed node should be Offline");

    // Test case 6: Edge case - just above threshold
    let gateway = create_test_gateway(76); // 76% mixnet performance
    let probe = create_test_probe_outcome(2.1, 0.75); // 2.1 Mbps (0.75 score), 75% ping
    let score = calculate_score(&gateway, &probe);
    // Weighted: (0.76 * 0.4) + (0.75 * 0.3) + (0.75 * 0.3) = 0.304 + 0.225 + 0.225 = 0.754
    assert_eq!(
        score,
        ScoreValue::High,
        "Edge case just above 0.75 threshold should be High"
    );
}

/// Smoke test to ensure conversion from nym_gateway_probe crate's ProbeResult
/// to this crate's ProbeResult doesn't silently drop or default fields when
/// nym_gateway_probe types change.
///
/// All values are set to non-default (booleans=true, numbers non-zero, strings non-empty)
/// to catch cases where new fields might be left as default after conversion.
#[test]
fn conversion_from_gw_probe_latest() {
    use nym_gateway_probe::types::{
        Entry as EntryLatest, EntryTestResult as EntryTestResultLatest, Exit as ExitLatest,
        ProbeOutcome as ProbeOutcomeLatest, ProbeResult as ProbeResultLatest,
        WgProbeResults as WgProbeResultsLatest,
    };

    // Build a ProbeResultLatest with ALL non-default values
    let wg_latest = WgProbeResultsLatest {
        can_register: true,
        can_query_metadata_v4: true,
        can_handshake_v4: true,
        can_resolve_dns_v4: true,
        ping_hosts_performance_v4: 0.95,
        ping_ips_performance_v4: 0.92,
        can_handshake_v6: true,
        can_resolve_dns_v6: true,
        ping_hosts_performance_v6: 0.88,
        ping_ips_performance_v6: 0.85,
        download_duration_sec_v4: 5,
        download_duration_milliseconds_v4: 5123,
        downloaded_file_size_bytes_v4: 10485760,
        downloaded_file_v4: "test-file-v4.bin".to_string(),
        download_error_v4: "none-v4".to_string(),
        download_duration_sec_v6: 6,
        download_duration_milliseconds_v6: 6234,
        downloaded_file_size_bytes_v6: 20971520,
        downloaded_file_v6: "test-file-v6.bin".to_string(),
        download_error_v6: "none-v6".to_string(),
    };
    let probe_latest = ProbeResultLatest {
        node: "test-node-identity-key".to_string(),
        used_entry: "test-entry-node".to_string(),
        outcome: ProbeOutcomeLatest {
            as_entry: EntryLatest::Tested(EntryTestResultLatest {
                can_connect: true,
                can_route: true,
            }),
            as_exit: Some(ExitLatest {
                can_connect: true,
                can_route_ip_v4: true,
                can_route_ip_external_v4: true,
                can_route_ip_v6: true,
                can_route_ip_external_v6: true,
            }),
            // TODO socks5 and lp fields
            socks5: None,
            lp: None,
            wg: Some(wg_latest.clone()),
        },
    };

    // convert to this crate's LastProbeResult
    let result: LastProbeResult = probe_latest.clone().into();

    assert_eq!(result.node, probe_latest.node);
    assert_eq!(result.used_entry, probe_latest.used_entry);

    match &result.outcome.as_entry {
        Entry::Tested(entry) => {
            assert!(entry.can_connect);
            assert!(entry.can_route);
        }
        other => panic!("Expected Entry::Tested, got {:?}", other),
    }

    // Exit conversion
    let exit = result
        .outcome
        .as_exit
        .as_ref()
        .expect("as_exit should be Some");
    assert!(exit.can_connect);
    assert!(exit.can_route_ip_v4);
    assert!(exit.can_route_ip_external_v4,);
    assert!(exit.can_route_ip_v6);
    assert!(exit.can_route_ip_external_v6,);

    // WgProbeResults conversion
    let wg = result.outcome.wg.as_ref().expect("wg should be Some");
    assert!(wg.can_register);
    assert_eq!(wg.can_query_metadata_v4, Some(true),);
    assert!(wg.can_handshake_v4);
    assert!(wg.can_resolve_dns_v4,);
    assert_eq!(
        wg.ping_hosts_performance_v4,
        wg_latest.ping_hosts_performance_v4
    );
    assert_eq!(
        wg.ping_ips_performance_v4,
        wg_latest.ping_ips_performance_v4
    );
    assert!(wg.can_handshake_v6);
    assert!(wg.can_resolve_dns_v6);
    assert_eq!(
        wg.ping_hosts_performance_v6,
        wg_latest.ping_hosts_performance_v6
    );
    assert_eq!(
        wg.ping_ips_performance_v6,
        wg_latest.ping_ips_performance_v6
    );
    assert_eq!(
        wg.download_duration_sec_v4,
        wg_latest.download_duration_sec_v4
    );
    assert_eq!(
        wg.download_duration_milliseconds_v4,
        Some(wg_latest.download_duration_milliseconds_v4),
    );
    assert_eq!(
        wg.downloaded_file_size_bytes_v4,
        Some(wg_latest.downloaded_file_size_bytes_v4),
    );
    assert_eq!(wg.downloaded_file_v4, wg_latest.downloaded_file_v4);
    assert_eq!(wg.download_error_v4, wg_latest.download_error_v4);
    assert_eq!(
        wg.download_duration_sec_v6,
        wg_latest.download_duration_sec_v6
    );
    assert_eq!(
        wg.download_duration_milliseconds_v6,
        Some(wg_latest.download_duration_milliseconds_v6)
    );
    assert_eq!(
        wg.downloaded_file_size_bytes_v6,
        Some(wg_latest.downloaded_file_size_bytes_v6)
    );
    assert_eq!(wg.downloaded_file_v6, wg_latest.downloaded_file_v6);
    assert_eq!(wg.download_error_v6, wg_latest.download_error_v6);

    // fields that map from v4 values
    assert_eq!(wg.can_handshake, Some(true));
    assert_eq!(wg.can_resolve_dns, Some(true));
    assert_eq!(
        wg.ping_hosts_performance,
        Some(wg_latest.ping_hosts_performance_v4)
    );
    assert_eq!(
        wg.ping_ips_performance,
        Some(wg_latest.ping_ips_performance_v4)
    );
}

#[test]
fn conversion_entry_variants() {
    use nym_gateway_probe::types::Entry as EntryLatest;

    let not_tested: Entry = EntryLatest::NotTested.into();
    assert!(matches!(not_tested, Entry::NotTested));

    let failure: Entry = EntryLatest::EntryFailure.into();
    assert!(matches!(failure, Entry::EntryFailure));
}

/// Backwards compatibility: this crate's struct may be present in DB of
/// some gateways even after the new nym_gateway_probe format is published.
/// DB entry needs to stay deserializable into a valid struct.
#[test]
fn deserialize_this_crate_format() {
    // JSON that matches this crate's ProbeResult format (not nym_gateway_probe)
    let old_format_json = serde_json::json!({
        "node": "old-node-key",
        "used_entry": "old-entry-key",
        "outcome": {
            "as_entry": {
                "can_connect": true,
                "can_route": false
            },
            "as_exit": null,
            "wg": null
        }
    });

    let result = LastProbeResult::deserialize_with_fallback(old_format_json)
        .expect("Should deserialize old format");

    assert_eq!(result.node, "old-node-key");
    assert_eq!(result.used_entry, "old-entry-key");
    match &result.outcome.as_entry {
        Entry::Tested(entry) => {
            assert!(entry.can_connect);
            assert!(!entry.can_route);
        }
        other => panic!("Expected Entry::Tested, got {:?}", other),
    }
    assert!(result.outcome.as_exit.is_none());
    assert!(result.outcome.wg.is_none());
}

/// Test that the latest nym_gateway_probe format deserializes correctly
#[test]
fn deserialize_latest_gw_probe_format() {
    // JSON that matches nym_gateway_probe::types::ProbeResult format
    let latest_format_json = serde_json::json!({
        "node": "new-node-key",
        "used_entry": "new-entry-key",
        "outcome": {
            "as_entry": {
                "can_connect": true,
                "can_route": true
            },
            "as_exit": {
                "can_connect": true,
                "can_route_ip_v4": true,
                "can_route_ip_external_v4": true,
                "can_route_ip_v6": false,
                "can_route_ip_external_v6": false
            },
            "socks5": null,
            "lp": null,
            "wg": {
                "can_register": true,
                "can_query_metadata_v4": true,
                "can_handshake_v4": true,
                "can_resolve_dns_v4": true,
                "ping_hosts_performance_v4": 0.9,
                "ping_ips_performance_v4": 0.85,
                "can_handshake_v6": false,
                "can_resolve_dns_v6": false,
                "ping_hosts_performance_v6": 0.0,
                "ping_ips_performance_v6": 0.0,
                "download_duration_sec_v4": 3,
                "download_duration_milliseconds_v4": 3456,
                "downloaded_file_size_bytes_v4": 5242880,
                "downloaded_file_v4": "5mb.bin",
                "download_error_v4": "",
                "download_duration_sec_v6": 0,
                "download_duration_milliseconds_v6": 0,
                "downloaded_file_size_bytes_v6": 0,
                "downloaded_file_v6": "",
                "download_error_v6": "ipv6 not supported"
            }
        }
    });

    let result = LastProbeResult::deserialize_with_fallback(latest_format_json)
        .expect("Should deserialize latest format");

    assert_eq!(result.node, "new-node-key");
    assert_eq!(result.used_entry, "new-entry-key");

    let exit = result.outcome.as_exit.as_ref().expect("should have exit");
    assert!(exit.can_route_ip_external_v4);
    assert!(!exit.can_route_ip_external_v6);

    let wg = result.outcome.wg.as_ref().expect("should have wg");
    assert!(wg.can_register);
    assert_eq!(wg.download_duration_milliseconds_v4, Some(3456));
    assert_eq!(wg.download_error_v6, "ipv6 not supported");
}

/// Serialize LastProbeResult to JSON and back to ensure serde attributes
/// work correctly and no data is lost.
#[test]
fn round_trip_serialization() {
    let original = LastProbeResult {
        node: "round-trip-node".to_string(),
        used_entry: "round-trip-entry".to_string(),
        outcome: ProbeOutcome {
            as_entry: Entry::Tested(EntryTestResult {
                can_connect: true,
                can_route: true,
            }),
            as_exit: Some(Exit {
                can_connect: true,
                can_route_ip_v4: true,
                can_route_ip_external_v4: true,
                can_route_ip_v6: true,
                can_route_ip_external_v6: true,
            }),
            wg: Some(WgProbeResults {
                can_register: true,
                can_handshake: Some(true),
                can_resolve_dns: Some(true),
                ping_hosts_performance: Some(0.95),
                ping_ips_performance: Some(0.92),
                can_query_metadata_v4: Some(true),
                can_handshake_v4: true,
                can_resolve_dns_v4: true,
                ping_hosts_performance_v4: 0.95,
                ping_ips_performance_v4: 0.92,
                can_handshake_v6: true,
                can_resolve_dns_v6: true,
                ping_hosts_performance_v6: 0.88,
                ping_ips_performance_v6: 0.85,
                download_duration_sec_v4: 5,
                download_duration_milliseconds_v4: Some(5123),
                downloaded_file_size_bytes_v4: Some(10485760),
                downloaded_file_v4: "test-file-v4.bin".to_string(),
                download_error_v4: "none-v4".to_string(),
                download_duration_sec_v6: 6,
                download_duration_milliseconds_v6: Some(6234),
                downloaded_file_size_bytes_v6: Some(20971520),
                downloaded_file_v6: "test-file-v6.bin".to_string(),
                download_error_v6: "none-v6".to_string(),
            }),
        },
    };

    // Serialize to JSON
    let json_string =
        serde_json::to_string(&original).expect("Should serialize LastProbeResult to JSON");

    // Deserialize back
    let deserialized: LastProbeResult =
        serde_json::from_str(&json_string).expect("Should deserialize JSON to LastProbeResult");

    // Verify top-level fields
    assert_eq!(deserialized.node, original.node);
    assert_eq!(deserialized.used_entry, original.used_entry);

    // Verify Entry
    match (&original.outcome.as_entry, &deserialized.outcome.as_entry) {
        (Entry::Tested(orig), Entry::Tested(deser)) => {
            assert_eq!(orig.can_connect, deser.can_connect);
            assert_eq!(orig.can_route, deser.can_route);
        }
        _ => panic!("Entry mismatch after round-trip"),
    }

    // Verify Exit
    let orig_exit = original.outcome.as_exit.as_ref().unwrap();
    let deser_exit = deserialized.outcome.as_exit.as_ref().unwrap();
    assert_eq!(orig_exit.can_connect, deser_exit.can_connect);
    assert_eq!(orig_exit.can_route_ip_v4, deser_exit.can_route_ip_v4);
    assert_eq!(
        orig_exit.can_route_ip_external_v4,
        deser_exit.can_route_ip_external_v4
    );
    assert_eq!(orig_exit.can_route_ip_v6, deser_exit.can_route_ip_v6);
    assert_eq!(
        orig_exit.can_route_ip_external_v6,
        deser_exit.can_route_ip_external_v6
    );

    // Verify WgProbeResults
    let orig_wg = original.outcome.wg.as_ref().unwrap();
    let deser_wg = deserialized.outcome.wg.as_ref().unwrap();
    assert_eq!(orig_wg.can_register, deser_wg.can_register);
    assert_eq!(orig_wg.can_handshake, deser_wg.can_handshake);
    assert_eq!(orig_wg.can_resolve_dns, deser_wg.can_resolve_dns);
    assert_eq!(
        orig_wg.ping_hosts_performance,
        deser_wg.ping_hosts_performance
    );
    assert_eq!(orig_wg.ping_ips_performance, deser_wg.ping_ips_performance);
    assert_eq!(
        orig_wg.can_query_metadata_v4,
        deser_wg.can_query_metadata_v4
    );
    assert_eq!(orig_wg.can_handshake_v4, deser_wg.can_handshake_v4);
    assert_eq!(orig_wg.can_resolve_dns_v4, deser_wg.can_resolve_dns_v4);
    assert_eq!(
        orig_wg.ping_hosts_performance_v4,
        deser_wg.ping_hosts_performance_v4
    );
    assert_eq!(
        orig_wg.ping_ips_performance_v4,
        deser_wg.ping_ips_performance_v4
    );
    assert_eq!(orig_wg.can_handshake_v6, deser_wg.can_handshake_v6);
    assert_eq!(orig_wg.can_resolve_dns_v6, deser_wg.can_resolve_dns_v6);
    assert_eq!(
        orig_wg.ping_hosts_performance_v6,
        deser_wg.ping_hosts_performance_v6
    );
    assert_eq!(
        orig_wg.ping_ips_performance_v6,
        deser_wg.ping_ips_performance_v6
    );
    assert_eq!(
        orig_wg.download_duration_sec_v4,
        deser_wg.download_duration_sec_v4
    );
    assert_eq!(
        orig_wg.download_duration_milliseconds_v4,
        deser_wg.download_duration_milliseconds_v4
    );
    assert_eq!(
        orig_wg.downloaded_file_size_bytes_v4,
        deser_wg.downloaded_file_size_bytes_v4
    );
    assert_eq!(orig_wg.downloaded_file_v4, deser_wg.downloaded_file_v4);
    assert_eq!(orig_wg.download_error_v4, deser_wg.download_error_v4);
    assert_eq!(
        orig_wg.download_duration_sec_v6,
        deser_wg.download_duration_sec_v6
    );
    assert_eq!(
        orig_wg.download_duration_milliseconds_v6,
        deser_wg.download_duration_milliseconds_v6
    );
    assert_eq!(
        orig_wg.downloaded_file_size_bytes_v6,
        deser_wg.downloaded_file_size_bytes_v6
    );
    assert_eq!(orig_wg.downloaded_file_v6, deser_wg.downloaded_file_v6);
    assert_eq!(orig_wg.download_error_v6, deser_wg.download_error_v6);
}
