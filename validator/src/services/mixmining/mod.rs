// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use db::MixminingDb;
use models::*;

pub mod db;
pub mod models;
mod tests;

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
    pub fn add(&mut self, mixnode: Mixnode) {
        self.db.add(mixnode);
    }

    pub fn topology(&self) -> Topology {
        let mixnodes = self.db.get_mixnodes();
        let service_providers: Vec<ServiceProvider> = vec![];
        let validators: Vec<Validator> = vec![];
        Topology::new(mixnodes.to_vec(), service_providers, validators)
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.db.set_capacity(capacity);
    }

    /// A fake capacity, so we can take the top n mixnodes based on stake
    pub fn capacity(&self) -> usize {
        self.db.capacity()
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

#[cfg(test)]
mod mixnodes {
    use super::*;

    #[test]
    fn adding_and_retrieving_works() {
        let mock_db = MixminingDb::new();
        let mut service = Service::new(mock_db);
        let node1 = tests::fake_mixnode("London, UK");

        service.add(node1.clone());
        let nodes = service.topology().mixnodes;
        assert_eq!(1, nodes.len());
        assert_eq!(node1.clone(), nodes[0]);
        let node2 = tests::fake_mixnode("Neuchatel");

        service.add(node2.clone());
        let nodes = service.topology().mixnodes;
        assert_eq!(2, nodes.len());
        assert_eq!(node1, nodes[0]);
        assert_eq!(node2, nodes[1]);
    }
}

#[cfg(test)]
mod constructor {
    use super::*;

    #[test]
    fn sets_database() {
        let db = db::MixminingDb::new();
        let service = Service::new(db.clone());

        assert_eq!(db, service.db);
    }
}

#[cfg(test)]
mod capacity {
    use super::*;

    #[test]
    fn setting_capacity_sends_correct_value_to_datastore() {
        let mock_db = db::MixminingDb::new();
        let mut service = Service::new(mock_db);

        service.set_capacity(3);

        assert_eq!(3, service.capacity());
    }

    #[test]
    fn getting_capacity_works() {
        let mut mock_db = db::MixminingDb::new();
        mock_db.set_capacity(3);
        let service = Service::new(mock_db);
        assert_eq!(3, service.capacity());
    }
}
