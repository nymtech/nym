use crate::support::storage::NymApiStorage;
use mixnet_contract_common::{reward_params::Performance, Interval, MixId};
use mixnet_contract_common::{MixNodeDetails, RewardedSetNodeStatus};
use nym_api_requests::models::MixNodeBondAnnotated;
use std::collections::HashMap;

pub(super) fn to_rewarded_set_node_status(
    rewarded_set: &[MixNodeDetails],
    active_set: &[MixNodeDetails],
) -> HashMap<MixId, RewardedSetNodeStatus> {
    let mut rewarded_set_node_status: HashMap<MixId, RewardedSetNodeStatus> = rewarded_set
        .iter()
        .map(|m| (m.mix_id(), RewardedSetNodeStatus::Standby))
        .collect();
    for mixnode in active_set {
        *rewarded_set_node_status
            .get_mut(&mixnode.mix_id())
            .expect("All active nodes are rewarded nodes") = RewardedSetNodeStatus::Active;
    }
    rewarded_set_node_status
}

pub(super) fn split_into_active_and_rewarded_set(
    mixnodes_annotated: &[MixNodeBondAnnotated],
    rewarded_set_node_status: &HashMap<u32, RewardedSetNodeStatus>,
) -> (Vec<MixNodeBondAnnotated>, Vec<MixNodeBondAnnotated>) {
    let rewarded_set: Vec<_> = mixnodes_annotated
        .iter()
        .filter(|mixnode| rewarded_set_node_status.get(&mixnode.mix_id()).is_some())
        .cloned()
        .collect();
    let active_set: Vec<_> = rewarded_set
        .iter()
        .filter(|mixnode| {
            rewarded_set_node_status
                .get(&mixnode.mix_id())
                .map_or(false, RewardedSetNodeStatus::is_active)
        })
        .cloned()
        .collect();
    (rewarded_set, active_set)
}

pub(super) async fn get_performance_from_storage(
    storage: &Option<NymApiStorage>,
    mix_id: MixId,
    epoch: Interval,
) -> Option<Performance> {
    storage
        .as_ref()?
        .get_average_mixnode_uptime_in_the_last_24hrs(
            mix_id,
            epoch.current_epoch_end_unix_timestamp(),
        )
        .await
        .ok()
        .map(Into::into)
}
