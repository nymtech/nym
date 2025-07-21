use serde::{Deserialize, Serialize};

#[allow(dead_code)] // it's not dead code but clippy doesn't detect usage in sqlx macros
#[derive(Debug, Clone)]
pub struct GatewayIdentityDto {
    pub gateway_identity_key: String,
    pub bonded: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct TestRun {
    pub id: i32,
    pub identity_key: String,
    pub status: String,
    pub log: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_identity_dto_creation() {
        let dto = GatewayIdentityDto {
            gateway_identity_key: "gateway123".to_string(),
            bonded: true,
        };

        assert_eq!(dto.gateway_identity_key, "gateway123");
        assert!(dto.bonded);
    }

    #[test]
    fn gateway_identity_dto_unbonded() {
        let dto = GatewayIdentityDto {
            gateway_identity_key: "gateway456".to_string(),
            bonded: false,
        };

        assert_eq!(dto.gateway_identity_key, "gateway456");
        assert!(!dto.bonded);
    }

    #[test]
    fn gateway_identity_dto_clone() {
        let original = GatewayIdentityDto {
            gateway_identity_key: "gateway789".to_string(),
            bonded: true,
        };

        let cloned = original.clone();

        assert_eq!(cloned.gateway_identity_key, original.gateway_identity_key);
        assert_eq!(cloned.bonded, original.bonded);
    }

    #[test]
    fn test_run_creation() {
        let test_run = TestRun {
            id: 1,
            identity_key: "test_gateway_123".to_string(),
            status: "success".to_string(),
            log: "Test completed successfully".to_string(),
        };

        assert_eq!(test_run.id, 1);
        assert_eq!(test_run.identity_key, "test_gateway_123");
        assert_eq!(test_run.status, "success");
        assert_eq!(test_run.log, "Test completed successfully");
    }

    #[test]
    fn test_run_with_error_status() {
        let test_run = TestRun {
            id: 42,
            identity_key: "error_gateway".to_string(),
            status: "error".to_string(),
            log: "Connection timeout: failed to reach gateway".to_string(),
        };

        assert_eq!(test_run.id, 42);
        assert_eq!(test_run.status, "error");
        assert!(test_run.log.contains("Connection timeout"));
    }

    #[test]
    fn test_run_serialization() {
        let test_run = TestRun {
            id: 123,
            identity_key: "serialization_test".to_string(),
            status: "pending".to_string(),
            log: "".to_string(),
        };

        // Test that it can be serialized
        let serialized = serde_json::to_string(&test_run).unwrap();
        assert!(serialized.contains("\"id\":123"));
        assert!(serialized.contains("\"identity_key\":\"serialization_test\""));
        assert!(serialized.contains("\"status\":\"pending\""));
        assert!(serialized.contains("\"log\":\"\""));

        // Test deserialization
        let deserialized: TestRun = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, test_run.id);
        assert_eq!(deserialized.identity_key, test_run.identity_key);
        assert_eq!(deserialized.status, test_run.status);
        assert_eq!(deserialized.log, test_run.log);
    }

    #[test]
    fn test_run_with_long_log() {
        let long_log = "Error: ".to_string() + &"x".repeat(1000);
        let test_run = TestRun {
            id: i32::MAX,
            identity_key: "long_log_test".to_string(),
            status: "failed".to_string(),
            log: long_log.clone(),
        };

        assert_eq!(test_run.id, i32::MAX);
        assert_eq!(test_run.log.len(), 1007); // "Error: " + 1000 x's
        assert_eq!(test_run.log, long_log);
    }

    #[test]
    fn test_run_with_special_characters() {
        let test_run = TestRun {
            id: 0,
            identity_key: "special_chars_∞_√_π".to_string(),
            status: "unknown".to_string(),
            log: "Test with\nnewlines\ttabs\rand \"quotes\"".to_string(),
        };

        assert_eq!(test_run.id, 0);
        assert!(test_run.identity_key.contains('∞'));
        assert!(test_run.log.contains('\n'));
        assert!(test_run.log.contains('\t'));
        assert!(test_run.log.contains('"'));
    }

    #[test]
    fn test_run_clone() {
        let original = TestRun {
            id: 999,
            identity_key: "clone_test".to_string(),
            status: "running".to_string(),
            log: "In progress...".to_string(),
        };

        let cloned = original.clone();

        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.identity_key, original.identity_key);
        assert_eq!(cloned.status, original.status);
        assert_eq!(cloned.log, original.log);
    }

    #[test]
    fn test_run_edge_cases() {
        // Test with negative ID (edge case)
        let test_run = TestRun {
            id: -1,
            identity_key: "".to_string(),
            status: "".to_string(),
            log: "".to_string(),
        };

        assert_eq!(test_run.id, -1);
        assert!(test_run.identity_key.is_empty());
        assert!(test_run.status.is_empty());
        assert!(test_run.log.is_empty());
    }
}
