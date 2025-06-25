// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[cfg(test)]
mod simulation_storage_tests {
    use super::super::models::{
        SimulatedNodePerformance, SimulatedPerformanceComparison, SimulatedRouteAnalysis,
    };
    use super::super::NymApiStorage;
    use tempfile::NamedTempFile;

    async fn create_test_storage() -> NymApiStorage {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Keep the temp file alive
        Box::leak(Box::new(temp_file));

        NymApiStorage::init(temp_path).await.unwrap()
    }

    #[tokio::test]
    async fn test_simulated_reward_epoch_crud() {
        let storage = create_test_storage().await;

        // Test create
        let (epoch_id, is_new) = storage
            .manager
            .create_or_get_simulated_reward_epoch(
                100,
                "test",
                1234567890,
                1234571490,
                Some("Test description"),
            )
            .await
            .unwrap();
        assert!(is_new);

        assert!(epoch_id > 0);

        // Test retrieve
        let retrieved = storage
            .manager
            .get_simulated_reward_epoch(epoch_id)
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let epoch = retrieved.unwrap();
        assert_eq!(epoch.epoch_id, 100);
        assert_eq!(epoch.calculation_method, "test");
        assert_eq!(epoch.start_timestamp, 1234567890);
        assert_eq!(epoch.end_timestamp, 1234571490);
        assert_eq!(epoch.description, Some("Test description".to_string()));

        // Test duplicate prevention
        let (duplicate_epoch_id, is_duplicate_new) = storage
            .manager
            .create_or_get_simulated_reward_epoch(
                100,
                "test",
                1234567890,
                1234571490,
                Some("Duplicate attempt"),
            )
            .await
            .unwrap();

        assert!(!is_duplicate_new);
        assert_eq!(duplicate_epoch_id, epoch_id); // Should return the same ID

        // Different method should create new entry
        let (different_method_id, is_different_new) = storage
            .manager
            .create_or_get_simulated_reward_epoch(
                100,
                "new",
                1234567890,
                1234571490,
                Some("Different method"),
            )
            .await
            .unwrap();

        assert!(is_different_new);
        assert_ne!(different_method_id, epoch_id); // Should be a different ID
    }

    #[tokio::test]
    async fn test_simulated_node_performance_crud() {
        let storage = create_test_storage().await;

        // Create parent epoch first
        let (epoch_id, _) = storage
            .manager
            .create_or_get_simulated_reward_epoch(100, "test", 1234567890, 1234571490, None)
            .await
            .unwrap();

        // Test insert node performance
        let performance = SimulatedNodePerformance {
            id: 0, // Will be set by database
            simulated_epoch_id: epoch_id,
            node_id: 42,
            node_type: "mixnode".to_string(),
            identity_key: Some("test_key".to_string()),
            reliability_score: 85.5,
            positive_samples: 100,
            negative_samples: 15,
            work_factor: Some(0.95),
            calculation_method: "new".to_string(),
            calculated_at: 1234567890,
        };

        storage
            .manager
            .insert_simulated_node_performance(&[performance])
            .await
            .unwrap();

        // Test retrieve
        let retrieved = storage
            .manager
            .get_simulated_node_performance_for_epoch(epoch_id)
            .await
            .unwrap();

        assert_eq!(retrieved.len(), 1);
        let perf = &retrieved[0];
        assert_eq!(perf.node_id, 42);
        assert_eq!(perf.node_type, "mixnode");
        assert_eq!(perf.reliability_score, 85.5);
        assert_eq!(perf.calculation_method, "new");
    }

    #[tokio::test]
    async fn test_simulated_performance_comparisons_crud() {
        let storage = create_test_storage().await;

        // Create parent epoch first
        let (epoch_id, _) = storage
            .manager
            .create_or_get_simulated_reward_epoch(100, "test", 1234567890, 1234571490, None)
            .await
            .unwrap();

        // Test insert performance comparison
        let comparison = SimulatedPerformanceComparison {
            id: 0,
            simulated_epoch_id: epoch_id,
            node_id: 42,
            node_type: "mixnode".to_string(),
            performance_score: 85.0,
            work_factor: 10.0,
            calculation_method: "new".to_string(),
            positive_samples: Some(100),
            negative_samples: Some(15),
            route_success_rate: Some(85.0),
            calculated_at: 1234567890,
        };

        storage
            .manager
            .insert_simulated_performance_comparisons(&[comparison])
            .await
            .unwrap();

        // Test retrieve
        let retrieved = storage
            .manager
            .get_simulated_performance_comparisons_for_epoch(epoch_id)
            .await
            .unwrap();

        assert_eq!(retrieved.len(), 1);
        let comp = &retrieved[0];
        assert_eq!(comp.node_id, 42);
        assert_eq!(comp.performance_score, 85.0);
        assert_eq!(comp.work_factor, 10.0);
        assert_eq!(comp.calculation_method, "new");
    }

    #[tokio::test]
    async fn test_simulated_route_analysis_crud() {
        let storage = create_test_storage().await;

        // Create parent epoch first
        let (epoch_id, _) = storage
            .manager
            .create_or_get_simulated_reward_epoch(100, "test", 1234567890, 1234571490, None)
            .await
            .unwrap();

        // Test insert route analysis
        let analysis = SimulatedRouteAnalysis {
            id: 0,
            simulated_epoch_id: epoch_id,
            calculation_method: "new".to_string(),
            total_routes_analyzed: 1000,
            successful_routes: 950,
            failed_routes: 50,
            average_route_reliability: Some(95.0),
            time_window_hours: 1,
            analysis_parameters: Some("{\"threshold\": 0.8}".to_string()),
            calculated_at: 1234567890,
        };

        storage
            .manager
            .insert_simulated_route_analysis(&analysis)
            .await
            .unwrap();

        // Test retrieve
        let retrieved = storage
            .manager
            .get_simulated_route_analysis_for_epoch(epoch_id)
            .await
            .unwrap();

        assert_eq!(retrieved.len(), 1);
        let route = &retrieved[0];
        assert_eq!(route.calculation_method, "new");
        assert_eq!(route.total_routes_analyzed, 1000);
        assert_eq!(route.successful_routes, 950);
        assert_eq!(route.average_route_reliability, Some(95.0));
        assert_eq!(route.time_window_hours, 1);
    }

    #[tokio::test]
    async fn test_foreign_key_constraints() {
        let storage = create_test_storage().await;

        // Try to insert node performance without valid epoch - should fail
        let performance = SimulatedNodePerformance {
            id: 0,
            simulated_epoch_id: 999999, // Non-existent epoch
            node_id: 42,
            node_type: "mixnode".to_string(),
            identity_key: None,
            reliability_score: 85.5,
            positive_samples: 100,
            negative_samples: 15,
            work_factor: Some(0.95),
            calculation_method: "new".to_string(),
            calculated_at: 1234567890,
        };

        let result = storage
            .manager
            .insert_simulated_node_performance(&[performance])
            .await;
        assert!(result.is_err()); // Should fail due to foreign key constraint
    }

    #[tokio::test]
    async fn test_performance_by_method_queries() {
        let storage = create_test_storage().await;

        // Create epoch
        let (epoch_id, _) = storage
            .manager
            .create_or_get_simulated_reward_epoch(100, "comparison", 1234567890, 1234571490, None)
            .await
            .unwrap();

        // Insert performance data for both methods
        let old_performance = SimulatedNodePerformance {
            id: 0,
            simulated_epoch_id: epoch_id,
            node_id: 42,
            node_type: "mixnode".to_string(),
            identity_key: Some("key42".to_string()),
            reliability_score: 80.0,
            positive_samples: 100,
            negative_samples: 20,
            work_factor: Some(0.9),
            calculation_method: "old".to_string(),
            calculated_at: 1234567890,
        };

        let new_performance = SimulatedNodePerformance {
            id: 0,
            simulated_epoch_id: epoch_id,
            node_id: 42,
            node_type: "mixnode".to_string(),
            identity_key: Some("key42".to_string()),
            reliability_score: 90.0,
            positive_samples: 95,
            negative_samples: 5,
            work_factor: Some(0.95),
            calculation_method: "new".to_string(),
            calculated_at: 1234567890,
        };

        storage
            .manager
            .insert_simulated_node_performance(&[old_performance])
            .await
            .unwrap();
        storage
            .manager
            .insert_simulated_node_performance(&[new_performance])
            .await
            .unwrap();

        // Test querying by method
        let old_results = storage
            .manager
            .get_simulated_node_performance_by_method(100, "old")
            .await
            .unwrap();

        let new_results = storage
            .manager
            .get_simulated_node_performance_by_method(100, "new")
            .await
            .unwrap();

        assert_eq!(old_results.len(), 1);
        assert_eq!(new_results.len(), 1);

        assert_eq!(old_results[0].reliability_score, 80.0);
        assert_eq!(new_results[0].reliability_score, 90.0);

        assert_eq!(old_results[0].calculation_method, "old");
        assert_eq!(new_results[0].calculation_method, "new");
    }

    #[tokio::test]
    async fn test_node_performance_history() {
        let storage = create_test_storage().await;

        // Create multiple epochs
        let (epoch1_id, _) = storage
            .manager
            .create_or_get_simulated_reward_epoch(100, "comparison", 1234567890, 1234571490, None)
            .await
            .unwrap();

        let (epoch2_id, _) = storage
            .manager
            .create_or_get_simulated_reward_epoch(101, "comparison", 1234571490, 1234575090, None)
            .await
            .unwrap();

        // Insert performance data for same node across epochs
        let performances = vec![
            SimulatedNodePerformance {
                id: 0,
                simulated_epoch_id: epoch1_id,
                node_id: 42,
                node_type: "mixnode".to_string(),
                identity_key: Some("key42".to_string()),
                reliability_score: 80.0,
                positive_samples: 100,
                negative_samples: 20,
                work_factor: Some(0.9),
                calculation_method: "old".to_string(),
                calculated_at: 1234567890,
            },
            SimulatedNodePerformance {
                id: 0,
                simulated_epoch_id: epoch2_id,
                node_id: 42,
                node_type: "mixnode".to_string(),
                identity_key: Some("key42".to_string()),
                reliability_score: 85.0,
                positive_samples: 105,
                negative_samples: 15,
                work_factor: Some(0.92),
                calculation_method: "old".to_string(),
                calculated_at: 1234571490,
            },
        ];

        for perf in performances {
            storage
                .manager
                .insert_simulated_node_performance(&[perf])
                .await
                .unwrap();
        }

        // Test node history query
        let history = storage
            .manager
            .get_simulated_node_performance_history(42)
            .await
            .unwrap();

        assert_eq!(history.len(), 2);

        // Should be ordered by epoch_id DESC
        assert_eq!(history[0].reliability_score, 85.0); // epoch 101
        assert_eq!(history[1].reliability_score, 80.0); // epoch 100
    }

    #[tokio::test]
    async fn test_count_operations() {
        let storage = create_test_storage().await;

        // Create epoch
        let (epoch_id, _) = storage
            .manager
            .create_or_get_simulated_reward_epoch(100, "test", 1234567890, 1234571490, None)
            .await
            .unwrap();

        // Initially should be 0
        let count = storage
            .manager
            .count_simulated_node_performance_for_epoch(epoch_id)
            .await
            .unwrap();
        assert_eq!(count, 0);

        // Insert some performance data
        let performances = vec![
            SimulatedNodePerformance {
                id: 0,
                simulated_epoch_id: epoch_id,
                node_id: 1,
                node_type: "mixnode".to_string(),
                identity_key: None,
                reliability_score: 80.0,
                positive_samples: 100,
                negative_samples: 20,
                work_factor: Some(0.9),
                calculation_method: "old".to_string(),
                calculated_at: 1234567890,
            },
            SimulatedNodePerformance {
                id: 0,
                simulated_epoch_id: epoch_id,
                node_id: 2,
                node_type: "gateway".to_string(),
                identity_key: None,
                reliability_score: 90.0,
                positive_samples: 120,
                negative_samples: 10,
                work_factor: None,
                calculation_method: "new".to_string(),
                calculated_at: 1234567890,
            },
        ];

        for perf in performances {
            storage
                .manager
                .insert_simulated_node_performance(&[perf])
                .await
                .unwrap();
        }

        // Should now be 2
        let count = storage
            .manager
            .count_simulated_node_performance_for_epoch(epoch_id)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }
}
