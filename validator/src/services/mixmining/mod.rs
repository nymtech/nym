use db::MixminingDb;
use serde::{Deserialize, Serialize};

pub mod db;
pub mod health_check_runner;

pub struct Service {
    db: MixminingDb,
}

/// The mixmining::Service provides logic for updating and slashing mixnode
/// stake, retrieving lists of mixnodes based on stake, and adding/removing
/// mixnodes from the active set. It monitors mixnodes and rewards or slashes
/// based on the observed quality of service provided by a given mixnode.
///
/// Mixing and staking interact. Mixnodes first need to announce
/// their presence to the validators.
///
/// The mixnode then goes into the stack of available mixnodes.
///
/// However, it's not necessarily going to start actively mixing traffic.
/// That depends on how much stake is riding on it, and how much capacity the
/// network requires right now. We depend on the wisdom of stakers to put their
/// money on trustworthy mixnodes.
///
/// The active set of mixnodes will be able to expand or contract based on capacity.
/// For now, we simply take the top <capacity> nodes available, ordered by
/// <node.stake desc>.
///
/// A lot is going to need to change here. Commented code is here mainly to
/// quickly sketch out the guts of the mixmining and staking service. This is not the basis
/// of our real staking system quite yet - it's a way to start getting the system
/// to function with all the different node types to start talking to each other,
/// and will be dramatically reworked over the next few months.
impl Service {
    pub fn new(db: MixminingDb) -> Service {
        Service { db }
    }
    // Add a mixnode so that it becomes part of the possible mixnode set.
    pub fn add(&self, mixnode: Mixnode) {
        println!("Adding mixnode: {:?}", mixnode);
    }

    pub fn set_capacity(&mut self, capacity: u32) {
        self.db.capacity = capacity;
    }

    /// A fake capacity, so we can take the top n mixnodes based on stake
    fn capacity(&self) -> u32 {
        self.db.capacity
    }

    /*

    /// Update (or create) a given mixnode stake, identified by the mixnode's public key
    fn update(&self, public_key: &str, amount: u64) {
        // retrieve the given Mixnode from the database and update its stake
    }

    /// For now, we have no notion of measuring capacity. For now just use capacity().
    fn active_mixnodes(&self) -> Vec<Mixnode> {
        Vec::<Mixnode>::new()
        // hit the database
    }


    /// Remove a mixnode from the active set in a way that does not impact its stake.
    /// In a more built-out system, this method would mean:
    /// "mixnode x has done its job well and requested to leave, so it can be removed
    ///  at the end of an epoch."
    fn remove(&self, public_key: &str) {
        // free locked up stake back to originating stakeholder
        // remove the mixnode from the database
    }

    /// Add the given amount of stake to the given Mixnode. Presumably it has done
    /// its job well.
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
    */
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mixnode {
    pub host: String,
    pub public_key: String,
    pub last_seen: u64,
    pub location: String,
    pub stake: u64,
    pub version: String,
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

#[cfg(test)]
mod test_capacity {
    use super::*;

    #[test]
    fn setting_capacity_sends_correct_value_to_datastore() {
        let mock_db = db::MixminingDb::new();
        let mut service = Service::new(mock_db);
        let cap = 3;

        service.set_capacity(cap);

        assert_eq!(3, service.db.capacity);
    }

    #[test]
    fn test_getting_capacity() {
        let mut mock_db = db::MixminingDb::new();
        mock_db.capacity = 3;
        let service = Service::new(mock_db);
        assert_eq!(3, service.capacity());
    }
}
