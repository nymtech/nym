use crate::validator::health_check::path_check::PathChecker;
use crate::validator::health_check::score::NodeScore;
use crypto::identity::{
    DummyMixIdentityKeyPair, DummyMixIdentityPublicKey, MixnetIdentityKeyPair,
    MixnetIdentityPublicKey,
};
use log::{debug, error, warn};
use sphinx::route::NodeAddressBytes;
use std::collections::HashMap;
use std::fmt::{Error, Formatter};
use topology::NymTopology;

#[derive(Debug)]
pub(crate) struct HealthCheckResult(Vec<NodeScore>);

impl std::fmt::Display for HealthCheckResult {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "NETWORK HEALTH\n==============\n")?;
        self.0
            .iter()
            .for_each(|score| write!(f, "{}\n", score).unwrap());
        Ok(())
    }
}

impl HealthCheckResult {
    fn zero_score<T: NymTopology>(topology: T) -> Self {
        warn!("The network is unhealthy, could not send any packets - returning zero score!");
        let mixes = topology.get_mix_nodes();
        let providers = topology.get_mix_provider_nodes();

        let health = mixes
            .into_iter()
            .map(|node| NodeScore::from_mixnode(node))
            .chain(
                providers
                    .into_iter()
                    .map(|node| NodeScore::from_provider(node)),
            )
            .collect();

        HealthCheckResult(health)
    }

    pub async fn calculate<T: NymTopology>(topology: T, iterations: usize) -> Self {
        let all_paths = match topology.all_paths() {
            Ok(paths) => paths,
            Err(_) => return Self::zero_score(topology),
        };

        // create entries for all nodes
        let mut score_map = HashMap::new();
        topology.get_mix_nodes().into_iter().for_each(|node| {
            score_map.insert(node.get_pub_key_bytes(), NodeScore::from_mixnode(node));
        });

        topology
            .get_mix_provider_nodes()
            .into_iter()
            .for_each(|node| {
                score_map.insert(node.get_pub_key_bytes(), NodeScore::from_provider(node));
            });

        let ephemeral_keys = DummyMixIdentityKeyPair::new();
        let providers = topology.get_mix_provider_nodes();

        let path_checker = PathChecker::new(providers, ephemeral_keys).await;

        // do it as many times is specified in config
        for i in 0..iterations {
            debug!("running healthcheck iteration {} / {}", i + 1, iterations);
            for path in &all_paths {
                let path_status = path_checker.check_path(&path);
                for node in path {
                    // if value doesn't exist, something extremely weird must have happened
                    let current_score = score_map.get_mut(&node.pub_key.0);
                    if current_score.is_none() {
                        return Self::zero_score(topology);
                    }
                    let current_score = current_score.unwrap();
                    current_score.increase_packet_count(path_status);
                }
            }
        }

        HealthCheckResult(score_map.drain().map(|(_, v)| v).collect())
    }
}
