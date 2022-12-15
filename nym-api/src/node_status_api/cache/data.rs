use nym_api_requests::models::MixNodeBondAnnotated;

use crate::contract_cache::cache::Cache;

use super::inclusion_probabilities::InclusionProbabilities;

#[derive(Default)]
pub(crate) struct NodeStatusCacheInner {
    pub(crate) mixnodes_annotated: Cache<Vec<MixNodeBondAnnotated>>,
    pub(crate) rewarded_set_annotated: Cache<Vec<MixNodeBondAnnotated>>,
    pub(crate) active_set_annotated: Cache<Vec<MixNodeBondAnnotated>>,

    // Estimated active set inclusion probabilities from Monte Carlo simulation
    pub(crate) inclusion_probabilities: Cache<InclusionProbabilities>,
}

impl NodeStatusCacheInner {
    pub fn new() -> Self {
        Self::default()
    }
}
