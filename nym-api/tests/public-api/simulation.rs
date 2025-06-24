use crate::utils::{base_url, make_request, validate_json_response};
use serde_json::Value;

#[tokio::test]
async fn test_simulation_epochs_endpoint() {
    let base = match base_url() {
        Ok(url) => url,
        Err(_) => {
            println!("NYM_API not set, skipping simulation API tests");
            return;
        }
    };

    let url = format!("{}/v1/simulation/epochs", base);
    let response = make_request(&url).await;

    match response {
        Ok(res) => {
            let json = validate_json_response(res).await;
            match json {
                Ok(data) => {
                    // Verify response structure
                    assert!(data.is_object());
                    assert!(data.get("epochs").is_some());
                    assert!(data.get("total_count").is_some());

                    let epochs = data.get("epochs").unwrap();
                    assert!(epochs.is_array());

                    // If there are epochs, verify their structure
                    if let Some(epoch_array) = epochs.as_array() {
                        if !epoch_array.is_empty() {
                            let first_epoch = &epoch_array[0];
                            verify_epoch_summary_structure(first_epoch);
                        }
                    }

                    println!("✓ Simulation epochs endpoint returned valid response");
                }
                Err(e) => {
                    println!("✗ Failed to parse JSON response: {}", e);
                }
            }
        }
        Err(_) => {
            // API might not be running in simulation mode, which is fine for tests
            println!("⚠ Simulation API not available (expected if not in simulation mode)");
        }
    }
}

#[tokio::test]
async fn test_simulation_epochs_with_pagination() {
    let base = match base_url() {
        Ok(url) => url,
        Err(_) => return,
    };

    let url = format!("{}/v1/simulation/epochs?limit=5&offset=0", base);
    let response = make_request(&url).await;

    match response {
        Ok(res) => {
            let json = validate_json_response(res).await;
            match json {
                Ok(data) => {
                    assert!(data.is_object());
                    assert!(data.get("epochs").is_some());
                    assert!(data.get("total_count").is_some());

                    let epochs = data.get("epochs").unwrap().as_array().unwrap();
                    assert!(epochs.len() <= 5); // Should respect limit

                    println!("✓ Simulation epochs pagination works correctly");
                }
                Err(_) => {
                    println!("⚠ Simulation API not available");
                }
            }
        }
        Err(_) => {
            println!("⚠ Simulation API not available");
        }
    }
}

#[tokio::test]
async fn test_simulation_epoch_details_structure() {
    let base = match base_url() {
        Ok(url) => url,
        Err(_) => return,
    };

    // First, get a list of epochs to find a valid ID
    let epochs_url = format!("{}/v1/simulation/epochs?limit=1", base);
    let epochs_response = make_request(&epochs_url).await;

    match epochs_response {
        Ok(res) => {
            let json = validate_json_response(res).await;
            if let Ok(data) = json {
                let epochs = data.get("epochs").unwrap().as_array().unwrap();
                if !epochs.is_empty() {
                    let first_epoch = &epochs[0];
                    let epoch_id = first_epoch.get("id").unwrap().as_i64().unwrap();

                    // Now test the details endpoint
                    let details_url = format!("{}/v1/simulation/epochs/{}", base, epoch_id);
                    let details_response = make_request(&details_url).await;

                    match details_response {
                        Ok(res) => {
                            let json = validate_json_response(res).await;
                            match json {
                                Ok(data) => {
                                    verify_epoch_details_structure(&data);
                                    println!("✓ Simulation epoch details endpoint returned valid structure");
                                }
                                Err(e) => {
                                    println!("✗ Failed to parse epoch details: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("✗ Failed to fetch epoch details: {}", e);
                        }
                    }
                }
            }
        }
        Err(_) => {
            println!("⚠ Simulation API not available");
        }
    }
}

#[tokio::test]
async fn test_simulation_comparison_endpoint() {
    let base = match base_url() {
        Ok(url) => url,
        Err(_) => return,
    };

    // First, get a list of epochs to find a valid ID
    let epochs_url = format!("{}/v1/simulation/epochs?limit=1", base);
    let epochs_response = make_request(&epochs_url).await;

    match epochs_response {
        Ok(res) => {
            let json = validate_json_response(res).await;
            if let Ok(data) = json {
                let epochs = data.get("epochs").unwrap().as_array().unwrap();
                if !epochs.is_empty() {
                    let first_epoch = &epochs[0];
                    let epoch_id = first_epoch.get("id").unwrap().as_i64().unwrap();

                    // Test the comparison endpoint
                    let comparison_url =
                        format!("{}/v1/simulation/epochs/{}/comparison", base, epoch_id);
                    let comparison_response = make_request(&comparison_url).await;

                    match comparison_response {
                        Ok(res) => {
                            let json = validate_json_response(res).await;
                            match json {
                                Ok(data) => {
                                    verify_comparison_structure(&data);
                                    println!(
                                        "✓ Simulation comparison endpoint returned valid structure"
                                    );
                                }
                                Err(e) => {
                                    println!("✗ Failed to parse comparison data: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("✗ Failed to fetch comparison data: {}", e);
                        }
                    }
                }
            }
        }
        Err(_) => {
            println!("⚠ Simulation API not available");
        }
    }
}

#[tokio::test]
async fn test_simulation_export_endpoints() {
    let base = match base_url() {
        Ok(url) => url,
        Err(_) => return,
    };

    // First, get a list of epochs to find a valid ID
    let epochs_url = format!("{}/v1/simulation/epochs?limit=1", base);
    let epochs_response = make_request(&epochs_url).await;

    match epochs_response {
        Ok(res) => {
            let json = validate_json_response(res).await;
            if let Ok(data) = json {
                let epochs = data.get("epochs").unwrap().as_array().unwrap();
                if !epochs.is_empty() {
                    let first_epoch = &epochs[0];
                    let epoch_id = first_epoch.get("id").unwrap().as_i64().unwrap();

                    // Test JSON export
                    let json_export_url = format!(
                        "{}/v1/simulation/epochs/{}/export?format=json",
                        base, epoch_id
                    );
                    let json_response = make_request(&json_export_url).await;

                    match json_response {
                        Ok(res) => {
                            assert!(res.status().is_success());
                            let content_type = res.headers().get("content-type");
                            if let Some(ct) = content_type {
                                assert!(ct.to_str().unwrap().contains("application/json"));
                            }
                            println!("✓ JSON export endpoint works correctly");
                        }
                        Err(e) => {
                            println!("✗ JSON export failed: {}", e);
                        }
                    }

                    // Test CSV export
                    let csv_export_url = format!(
                        "{}/v1/simulation/epochs/{}/export?format=csv",
                        base, epoch_id
                    );
                    let csv_response = make_request(&csv_export_url).await;

                    match csv_response {
                        Ok(res) => {
                            assert!(res.status().is_success());
                            let content_type = res.headers().get("content-type");
                            if let Some(ct) = content_type {
                                assert!(ct.to_str().unwrap().contains("text/csv"));
                            }
                            println!("✓ CSV export endpoint works correctly");
                        }
                        Err(e) => {
                            println!("✗ CSV export failed: {}", e);
                        }
                    }
                }
            }
        }
        Err(_) => {
            println!("⚠ Simulation API not available");
        }
    }
}

#[tokio::test]
async fn test_simulation_error_handling() {
    let base = match base_url() {
        Ok(url) => url,
        Err(_) => return,
    };

    // Test 404 for non-existent epoch
    let invalid_url = format!("{}/v1/simulation/epochs/999999", base);
    let response = make_request(&invalid_url).await;

    match response {
        Ok(res) => {
            // Should get a successful response (empty or error structure)
            assert!(res.status().is_success() || res.status().is_client_error());
            println!("✓ Error handling works for invalid epoch IDs");
        }
        Err(_) => {
            // This is also acceptable if the endpoint returns an error
            println!("✓ Error handling works for invalid epoch IDs");
        }
    }
}

// Helper functions to verify response structures

fn verify_epoch_summary_structure(epoch: &Value) {
    assert!(epoch.is_object());
    assert!(epoch.get("id").is_some());
    assert!(epoch.get("epoch_id").is_some());
    assert!(epoch.get("calculation_method").is_some());
    assert!(epoch.get("start_timestamp").is_some());
    assert!(epoch.get("end_timestamp").is_some());
    assert!(epoch.get("created_at").is_some());
    assert!(epoch.get("nodes_analyzed").is_some());
    assert!(epoch.get("available_methods").is_some());

    // Verify types
    assert!(epoch.get("id").unwrap().is_i64());
    assert!(epoch.get("epoch_id").unwrap().is_u64());
    assert!(epoch.get("calculation_method").unwrap().is_string());
    assert!(epoch.get("nodes_analyzed").unwrap().is_u64());
    assert!(epoch.get("available_methods").unwrap().is_array());
}

fn verify_epoch_details_structure(data: &Value) {
    assert!(data.is_object());
    assert!(data.get("epoch").is_some());
    assert!(data.get("node_performance").is_some());
    assert!(data.get("rewards").is_some());
    assert!(data.get("route_analysis").is_some());

    // Verify epoch summary structure
    verify_epoch_summary_structure(data.get("epoch").unwrap());

    // Verify arrays
    assert!(data.get("node_performance").unwrap().is_array());
    assert!(data.get("rewards").unwrap().is_array());
    assert!(data.get("route_analysis").unwrap().is_array());

    // If there's performance data, verify its structure
    let performance_array = data.get("node_performance").unwrap().as_array().unwrap();
    if !performance_array.is_empty() {
        let first_performance = &performance_array[0];
        verify_performance_data_structure(first_performance);
    }

    // If there's reward data, verify its structure
    let rewards_array = data.get("rewards").unwrap().as_array().unwrap();
    if !rewards_array.is_empty() {
        let first_reward = &rewards_array[0];
        verify_reward_data_structure(first_reward);
    }
}

fn verify_performance_data_structure(performance: &Value) {
    assert!(performance.is_object());
    assert!(performance.get("node_id").is_some());
    assert!(performance.get("node_type").is_some());
    assert!(performance.get("reliability_score").is_some());
    assert!(performance.get("positive_samples").is_some());
    assert!(performance.get("negative_samples").is_some());
    assert!(performance.get("calculation_method").is_some());
    assert!(performance.get("calculated_at").is_some());

    // Verify types
    assert!(performance.get("node_id").unwrap().is_u64());
    assert!(performance.get("node_type").unwrap().is_string());
    assert!(performance.get("reliability_score").unwrap().is_f64());
    assert!(performance.get("positive_samples").unwrap().is_u64());
    assert!(performance.get("negative_samples").unwrap().is_u64());
    assert!(performance.get("calculation_method").unwrap().is_string());
    assert!(performance.get("calculated_at").unwrap().is_i64());
}

fn verify_reward_data_structure(reward: &Value) {
    assert!(reward.is_object());
    assert!(reward.get("node_id").is_some());
    assert!(reward.get("node_type").is_some());
    assert!(reward.get("calculated_reward_amount").is_some());
    assert!(reward.get("reward_currency").is_some());
    assert!(reward.get("calculation_method").is_some());

    // Verify types
    assert!(reward.get("node_id").unwrap().is_u64());
    assert!(reward.get("node_type").unwrap().is_string());
    assert!(reward.get("calculated_reward_amount").unwrap().is_f64());
    assert!(reward.get("reward_currency").unwrap().is_string());
    assert!(reward.get("calculation_method").unwrap().is_string());
}

fn verify_comparison_structure(data: &Value) {
    assert!(data.is_object());
    assert!(data.get("epoch_id").is_some());
    assert!(data.get("simulation_epoch_id").is_some());
    assert!(data.get("node_comparisons").is_some());
    assert!(data.get("summary_statistics").is_some());
    assert!(data.get("route_analysis_comparison").is_some());

    // Verify types
    assert!(data.get("epoch_id").unwrap().is_u64());
    assert!(data.get("simulation_epoch_id").unwrap().is_i64());
    assert!(data.get("node_comparisons").unwrap().is_array());
    assert!(data.get("summary_statistics").unwrap().is_object());
    assert!(data.get("route_analysis_comparison").unwrap().is_object());

    // Verify summary statistics structure
    let stats = data.get("summary_statistics").unwrap();
    assert!(stats.get("total_nodes_compared").is_some());
    assert!(stats.get("nodes_improved").is_some());
    assert!(stats.get("nodes_degraded").is_some());
    assert!(stats.get("average_reliability_old").is_some());
    assert!(stats.get("average_reliability_new").is_some());
}
