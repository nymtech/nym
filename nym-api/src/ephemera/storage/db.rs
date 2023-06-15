use crate::ephemera::metrics::types::MixnodeResult;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::MixId;

pub struct Storage {}

impl Storage {
    pub fn init() -> Self {
        Self {}
    }
}

impl Storage {
    pub fn submit_mixnode_statuses(
        &mut self,
        _timestamp: i64,
        _mixnode_results: Vec<MixnodeResult>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn get_mixnode_average_reliability(
        &self,
        _id: MixId,
        _start: u64,
        _end: u64,
    ) -> anyhow::Result<Option<Performance>> {
        Ok(None)
    }

    pub fn save_rewarding_results(&mut self, _epoch: u64, _mixnodes: usize) -> anyhow::Result<()> {
        Ok(())
    }
}
