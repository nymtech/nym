// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[cfg(test)]
mod simulation_storage_tests {
    use super::super::manager::StorageManager;
    use super::super::models::{SimulatedNodePerformance, SimulatedReward, SimulatedRouteAnalysis};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
    use sqlx::ConnectOptions;
    use tempfile::NamedTempFile;
    use tokio::runtime::Runtime;
    
    async fn create_test_db() -> SqlitePool {
        let temp_file = NamedTempFile::new().unwrap();
        let db_url = format!("sqlite:{}", temp_file.path().display());
        
        let options = SqliteConnectOptions::new()
            .filename(temp_file.path())
            .create_if_missing(true);
            
        let pool = SqlitePool::connect_with(options).await.unwrap();
        
        // Run our actual migrations
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        
        pool
    }

    #[tokio::test]
    async fn test_simulated_reward_epoch_crud() {
        let pool = create_test_db().await;
        let storage = StorageManager { connection_pool: pool };

        // Test create
        let epoch_id = storage
            .create_simulated_reward_epoch(
                100,
                "test_method",
                1234567890,
                1234571490,
                Some("Test description"),
            )
            .await
            .unwrap();

        assert!(epoch_id > 0);

        // Test retrieve
        let retrieved = storage.get_simulated_reward_epoch(epoch_id).await.unwrap();
        assert!(retrieved.is_some());
        
        let epoch = retrieved.unwrap();
        assert_eq!(epoch.epoch_id, 100);
        assert_eq!(epoch.calculation_method, "test_method");
        assert_eq!(epoch.start_timestamp, 1234567890);
        assert_eq!(epoch.end_timestamp, 1234571490);
        assert_eq!(epoch.description, Some("Test description".to_string()));
    }

    #[tokio::test]
    async fn test_simulated_node_performance_crud() {
        let pool = create_test_db().await;
        let storage = StorageManager { connection_pool: pool };

        // Create parent epoch first
        let epoch_id = storage
            .create_simulated_reward_epoch(100, "test", 1234567890, 1234571490, None)
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
            final_fail_sequence: 2,
            work_factor: Some(0.95),
            calculation_method: "new".to_string(),
            calculated_at: 1234567890,
        };

        storage.insert_simulated_node_performance(&performance).await.unwrap();

        // Test retrieve
        let retrieved = storage
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
    async fn test_simulated_rewards_crud() {
        let pool = create_test_db().await;
        let storage = StorageManager { connection_pool: pool };

        // Create parent epoch first
        let epoch_id = storage
            .create_simulated_reward_epoch(100, "test", 1234567890, 1234571490, None)
            .await
            .unwrap();

        // Test insert reward
        let reward = SimulatedReward {
            id: 0,
            simulated_epoch_id: epoch_id,
            node_id: 42,
            node_type: "mixnode".to_string(),
            calculated_reward_amount: 1000.50,
            reward_currency: "nym".to_string(),
            performance_component: 85.0,
            work_component: 0.95,
            calculation_method: "new".to_string(),
            calculated_at: 1234567890,
        };

        storage.insert_simulated_reward(&reward).await.unwrap();

        // Test retrieve
        let retrieved = storage
            .get_simulated_rewards_for_epoch(epoch_id)
            .await
            .unwrap();

        assert_eq!(retrieved.len(), 1);
        let rew = &retrieved[0];
        assert_eq!(rew.node_id, 42);
        assert_eq!(rew.calculated_reward_amount, 1000.50);
        assert_eq!(rew.performance_component, 85.0);
        assert_eq!(rew.calculation_method, "new");
    }

    #[tokio::test]
    async fn test_simulated_route_analysis_crud() {
        let pool = create_test_db().await;
        let storage = StorageManager { connection_pool: pool };

        // Create parent epoch first
        let epoch_id = storage
            .create_simulated_reward_epoch(100, "test", 1234567890, 1234571490, None)
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

        storage.insert_simulated_route_analysis(&analysis).await.unwrap();

        // Test retrieve
        let retrieved = storage
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
        let pool = create_test_db().await;
        let storage = StorageManager { connection_pool: pool };

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
            final_fail_sequence: 2,
            work_factor: Some(0.95),
            calculation_method: "new".to_string(),
            calculated_at: 1234567890,
        };

        let result = storage.insert_simulated_node_performance(&performance).await;
        assert!(result.is_err()); // Should fail due to foreign key constraint
    }

    #[tokio::test]
    async fn test_performance_by_method_queries() {
        let pool = create_test_db().await;
        let storage = StorageManager { connection_pool: pool };

        // Create epoch
        let epoch_id = storage
            .create_simulated_reward_epoch(100, "comparison", 1234567890, 1234571490, None)
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
            final_fail_sequence: 1,
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
            final_fail_sequence: 0,
            work_factor: Some(0.95),
            calculation_method: "new".to_string(),
            calculated_at: 1234567890,
        };

        storage.insert_simulated_node_performance(&old_performance).await.unwrap();
        storage.insert_simulated_node_performance(&new_performance).await.unwrap();

        // Test querying by method
        let old_results = storage
            .get_simulated_node_performance_by_method(100, "old")
            .await
            .unwrap();
        
        let new_results = storage
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
        let pool = create_test_db().await;
        let storage = StorageManager { connection_pool: pool };

        // Create multiple epochs
        let epoch1_id = storage
            .create_simulated_reward_epoch(100, "comparison", 1234567890, 1234571490, None)
            .await
            .unwrap();
            
        let epoch2_id = storage
            .create_simulated_reward_epoch(101, "comparison", 1234571490, 1234575090, None)
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
                final_fail_sequence: 1,
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
                final_fail_sequence: 0,
                work_factor: Some(0.92),
                calculation_method: "old".to_string(),
                calculated_at: 1234571490,
            },
        ];

        for perf in performances {
            storage.insert_simulated_node_performance(&perf).await.unwrap();
        }

        // Test node history query
        let history = storage
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
        let pool = create_test_db().await;
        let storage = StorageManager { connection_pool: pool };

        // Create epoch
        let epoch_id = storage
            .create_simulated_reward_epoch(100, "test", 1234567890, 1234571490, None)
            .await
            .unwrap();

        // Initially should be 0
        let count = storage
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
                final_fail_sequence: 1,
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
                final_fail_sequence: 0,
                work_factor: None,
                calculation_method: "new".to_string(),
                calculated_at: 1234567890,
            },
        ];

        for perf in performances {
            storage.insert_simulated_node_performance(&perf).await.unwrap();
        }

        // Should now be 2
        let count = storage
            .count_simulated_node_performance_for_epoch(epoch_id)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }
}