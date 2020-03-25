use db::MixminingDb;
use serde::{Deserialize, Serialize};

pub mod db;
pub mod health_check_runner;

pub struct Service {
    db: MixminingDb,
}

/// The mixmining::StakeService provides logic for updating and slashing mixnode
/// stake, retrieving lists of mixnodes based on stake, and adding/removing
/// mixnodes from the active set.
///
/// Mixing and staking interact in interesting ways. Mixnodes first need to announce
/// their presence to the validators. The validators will then proceed to do a
/// health check on them.
///
/// Once a Mixnode passes its health check, it goes into the stack of available
/// mixnodes. However, it's not necessarily going to start actively mixing traffic.
/// That depends on how much stake is riding on it: we depend on the wisdom of
/// stakers to put their money on trustworthy mixnodes.
///
/// The active set of mixnodes will be able to expand or contract based on capacity
/// (not yet implemented). For now, we simply take the top N nodes available,
/// ordered by node stake.
///
/// A lot is going to need to change here. Commented code is here mainly to
/// quickly sketch out the guts of the staking service. This is not the basis
/// of our real staking system quite yet - it's a way to start getting the system
/// to function with all the different node types to start talking to each other,
/// and will be dramatically reworked over the next few months.
impl Service {
    pub fn new(db: MixminingDb) -> Service {
        Service { db }
    }
    // Add a mixnode so that it becomes part of the possible mixnode set.
    // Presumably it's passed health check.
    pub fn add(&self, mixnode: Mixnode) {
        println!("Add hit, mixnode: {:?}", mixnode);
    }

    /// Update (or create) a given mixnode stake, identified by the mixnode's public key
    fn update(&self, public_key: &str, amount: u64) {
        // retrieve the given Mixnode from the database and update its stake
    }

    /// For now, we have no notion of capacity. Return the top 6 mixnodes, ordered by stake.
    fn active_mixnodes(&self) -> Vec<Mixnode> {
        Vec::<Mixnode>::new()
        // hit the database
    }

    /// Remove a mixnode from the active set in a way that does not impact its stake.
    /// The mixnode has done its job well and requested to leave, so it can be removed
    ///  at the end of an epoch.
    fn remove(&self, public_key: &str) {
        // free locked up stake back to originating stakeholder
        // remove the mixnode from the database
    }

    // Add the given amount of stake to the given Mixnode. Presumably it has done
    // its job well.
    fn reward(&self, public_key: &str, amount: u64) {}

    /// Slash a mixnode's stake based on bad performance or detected malign intent.
    fn slash(&self, public_key: &str, amount: u64) {
        // transfer slashed stake amount to reserve fund
        // retrieve the mixnode from the database, and decrement its stake amount
        // by the amount given.
    }

    /// Slash a mixnode's stake and immediately remove it from the mixnode set.
    fn slash_remove(&self, public_key: String, amount: u64) {
        // call slash (the method, not the guitarist)
        // remove the mixnode from the database
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mixnode {
    pub public_key: String,
    pub stake: u64,
}

#[cfg(test)]
mod test_constructor {
    use super::*;

    #[test]
    fn test_constructor_sets_database() {
        let db = db::MixminingDb::new();
        let service = Service::new(db.clone());

        assert_eq!(db, service.db);
    }
}
