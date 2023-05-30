use crate::ephemera::contract::MixnodeToReward;
use crate::ephemera::epoch::EpochInfo;
use crate::ephemera::metrics::types::MixnodeResult;

#[derive(Clone)]
pub struct MetricsStorageType;

#[derive(Clone)]
pub struct ContractStorageType;

pub enum StorageType {
    Metrics,
    Contract,
}

pub struct Storage<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> Storage<T> {
    pub fn init() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl Storage<MetricsStorageType> {
    pub fn submit_mixnode_statuses(
        &mut self,
        _timestamp: i64,
        _mixnode_results: Vec<MixnodeResult>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn get_mixnode_average_reliability(
        &self,
        _id: usize,
        _start: u64,
        _end: u64,
    ) -> anyhow::Result<Option<f32>> {
        Ok(None)
    }

    pub fn save_rewarding_results(&mut self, _epoch: u64, _mixnodes: usize) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Storage<ContractStorageType> {
    pub fn contract_submit_mixnode_rewards(
        &mut self,
        _epoch: u64,
        _timestamp: i64,
        _nym_api_id: &str,
        _res: Vec<MixnodeToReward>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub(crate) fn save_epoch(&mut self, _info: &EpochInfo) -> anyhow::Result<()> {
        Ok(())
    }

    pub(crate) fn update_epoch(&mut self, _info: &EpochInfo) -> anyhow::Result<()> {
        Ok(())
    }

    pub(crate) fn get_epoch(&self) -> anyhow::Result<Option<EpochInfo>> {
        Ok(None)
    }
}
