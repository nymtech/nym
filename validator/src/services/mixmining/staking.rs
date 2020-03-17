pub struct StakeService {}

impl StakeService {
    fn update(stake: MixnodeStake) {
        // Update (or create) a given mixnode stake, identified by the mixnode's public key
    }
    fn active_mixnodes() {
        // For now, we have no notion of capacity. Return the top 6 mixnodes by stake.
    }
}

pub struct MixnodeStake {
    public_key: String,
    amount: u64,
}
