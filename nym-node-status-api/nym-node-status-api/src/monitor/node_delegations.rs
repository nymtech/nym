use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_validator_client::{nyxd::contract_traits::PagedMixnetQueryClient, QueryHttpRpcNyxdClient};
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::RwLock, time::Instant};
use tracing::{info, warn};

// abstracts away data structure that holds delegations
#[derive(Clone, Debug)]
pub(crate) struct DelegationsCache {
    pub inner: HashMap<NodeId, Vec<crate::http::models::NodeDelegation>>,
}

impl DelegationsCache {
    pub(crate) fn new() -> Arc<RwLock<Self>> {
        let a = Self {
            inner: HashMap::new(),
        };
        Arc::new(RwLock::new(a))
    }

    pub(crate) fn delegations_owned(
        &self,
        node_id: NodeId,
    ) -> Option<Vec<crate::http::models::NodeDelegation>> {
        self.inner.get(&node_id).cloned()
    }
}

pub(super) async fn refresh(
    client: &QueryHttpRpcNyxdClient,
    bonded_nodes: &HashMap<NodeId, NymNodeDetails>,
) -> DelegationsCache {
    info!("👥 Refreshing {} node delegations...", bonded_nodes.len());
    let now = Instant::now();

    let mut delegations_per_node = HashMap::new();
    for node_id in bonded_nodes.keys() {
        if let Ok(delegations) = client
            .get_all_single_mixnode_delegations(*node_id)
            .await
            .inspect_err(|err| warn!("Failed to get delegations for {}: {}", node_id, err))
        {
            delegations_per_node
                .insert(*node_id, delegations.into_iter().map(From::from).collect());
        }
    }
    let time_taken = Instant::now() - now;
    info!("👥 Node delegations refreshed in {}s", time_taken.as_secs(),);

    DelegationsCache {
        inner: delegations_per_node,
    }
}
