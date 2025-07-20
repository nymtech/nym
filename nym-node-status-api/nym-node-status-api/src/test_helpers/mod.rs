#[cfg(test)]
pub mod builders {
    use crate::http::models::*;
    use nym_validator_client::nym_api::SkimmedNode;
    use nym_validator_client::nym_nodes::NodeRole;
    use nym_validator_client::models::DeclaredRoles;
    use nym_contracts_common::Percent;
    use nym_crypto::asymmetric::{ed25519, x25519};
    
    /// Builder for creating test Gateway instances
    pub struct GatewayBuilder {
        gateway: Gateway,
    }
    
    impl GatewayBuilder {
        pub fn new() -> Self {
            Self {
                gateway: Gateway {
                    gateway_identity_key: "test_gateway".to_string(),
                    bonded: true,
                    performance: 95,
                    self_described: Some(serde_json::json!({})),
                    explorer_pretty_bond: Some(serde_json::json!({})),
                    description: nym_node_requests::api::v1::node::models::NodeDescription {
                        moniker: "Test Gateway".to_string(),
                        website: "https://example.com".to_string(),
                        security_contact: "admin@example.com".to_string(),
                        details: "Test gateway node".to_string(),
                    },
                    last_probe_result: None,
                    last_probe_log: None,
                    last_testrun_utc: None,
                    last_updated_utc: "2024-01-01T00:00:00Z".to_string(),
                    routing_score: 0.95,
                    config_score: 100,
                },
            }
        }
        
        pub fn with_identity(mut self, key: &str) -> Self {
            self.gateway.gateway_identity_key = key.to_string();
            self
        }
        
        pub fn with_performance(mut self, performance: u8) -> Self {
            self.gateway.performance = performance;
            self
        }
        
        pub fn unbonded(mut self) -> Self {
            self.gateway.bonded = false;
            self
        }
        
        pub fn with_last_probe_result(mut self, result: serde_json::Value) -> Self {
            self.gateway.last_probe_result = Some(result);
            self
        }
        
        pub fn build(self) -> Gateway {
            self.gateway
        }
    }
    
    /// Builder for creating test SkimmedNode instances
    pub struct SkimmedNodeBuilder {
        node: SkimmedNode,
    }
    
    impl SkimmedNodeBuilder {
        pub fn new() -> Self {
            let ed25519_pk = ed25519::PublicKey::from_bytes(&[1; 32]).unwrap();
            let x25519_pk = x25519::PublicKey::from_bytes(&[2; 32]).unwrap();
            
            Self {
                node: SkimmedNode {
                    node_id: 1,
                    ed25519_identity_pubkey: ed25519_pk,
                    ip_addresses: vec!["127.0.0.1".parse().unwrap()],
                    mix_port: 1789,
                    x25519_sphinx_pubkey: x25519_pk,
                    role: NodeRole::Mixnode { layer: 1 },
                    supported_roles: DeclaredRoles {
                        entry: false,
                        mixnode: true,
                        exit_nr: false,
                        exit_ipr: false,
                    },
                    entry: None,
                    performance: Percent::from_percentage_value(95).unwrap(),
                },
            }
        }
        
        pub fn with_node_id(mut self, id: u32) -> Self {
            self.node.node_id = id;
            self
        }
        
        pub fn with_role(mut self, role: NodeRole) -> Self {
            self.node.role = role;
            self
        }
        
        pub fn as_gateway(mut self) -> Self {
            self.node.role = NodeRole::EntryGateway;
            self.node.supported_roles = DeclaredRoles {
                entry: true,
                mixnode: false,
                exit_nr: true,
                exit_ipr: false,
            };
            self
        }
        
        pub fn with_performance(mut self, perf: u8) -> Self {
            self.node.performance = Percent::from_percentage_value(perf as u64).unwrap();
            self
        }
        
        pub fn build(self) -> SkimmedNode {
            self.node
        }
    }
    
    /// Builder for creating test Mixnode instances
    pub struct MixnodeBuilder {
        mixnode: Mixnode,
    }
    
    impl MixnodeBuilder {
        pub fn new() -> Self {
            Self {
                mixnode: Mixnode {
                    mix_id: 1,
                    bonded: true,
                    is_dp_delegatee: false,
                    total_stake: 1_000_000,
                    full_details: Some(serde_json::json!({})),
                    self_described: Some(serde_json::json!({})),
                    description: nym_node_requests::api::v1::node::models::NodeDescription {
                        moniker: "Test Mixnode".to_string(),
                        website: "https://example.com".to_string(),
                        security_contact: "admin@example.com".to_string(),
                        details: "Test mixnode".to_string(),
                    },
                    last_updated_utc: "2024-01-01T00:00:00Z".to_string(),
                },
            }
        }
        
        pub fn with_mix_id(mut self, id: u32) -> Self {
            self.mixnode.mix_id = id;
            self
        }
        
        pub fn with_stake(mut self, stake: i64) -> Self {
            self.mixnode.total_stake = stake;
            self
        }
        
        pub fn as_dp_delegatee(mut self) -> Self {
            self.mixnode.is_dp_delegatee = true;
            self
        }
        
        pub fn unbonded(mut self) -> Self {
            self.mixnode.bonded = false;
            self
        }
        
        pub fn build(self) -> Mixnode {
            self.mixnode
        }
    }
}