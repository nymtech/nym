use std::collections::HashSet;
use std::num::NonZeroUsize;

use log::warn;
use lru::LruCache;

use crate::peer::PeerId;
use crate::utilities::hash::Hash;

pub(crate) struct BroadcastGroup {
    /// The id of current group. Incremented every time a new snapshot is added.
    pub(crate) current_id: u64,
    /// A cache of the group snapshots.
    pub(crate) snapshots: LruCache<u64, HashSet<PeerId>>,
    /// A cache of the groups for each block.
    pub(crate) broadcast_groups: LruCache<Hash, u64>,
}

impl BroadcastGroup {
    pub(crate) fn new() -> BroadcastGroup {
        let mut snapshots = LruCache::new(NonZeroUsize::new(100).unwrap());
        snapshots.put(0, HashSet::new());
        BroadcastGroup {
            current_id: 0,
            snapshots,
            broadcast_groups: LruCache::new(NonZeroUsize::new(100).unwrap()),
        }
    }

    pub(crate) fn add_snapshot(&mut self, snapshot: HashSet<PeerId>) {
        self.current_id += 1;
        self.snapshots.put(self.current_id, snapshot);
    }

    pub(crate) fn is_member(&mut self, id: u64, peer_id: &PeerId) -> bool {
        self.snapshots
            .get(&id)
            .map_or(false, |s| s.contains(peer_id))
    }

    pub(crate) fn is_empty(&mut self) -> bool {
        self.snapshots
            .get(&self.current_id)
            .map_or(true, HashSet::is_empty)
    }

    // Returns empty snapshots(inserted in 'new' fn) if we haven't received any yet.
    pub(crate) fn current(&mut self) -> &HashSet<PeerId> {
        self.snapshots
            .get(&self.current_id)
            .expect("Current group should always exist")
    }

    // Checks if creator and sender are part of the expected group.
    // If we see hash first time, it checks against the current group. And if check passes, it
    // associates the hash with the current group.
    pub(crate) fn check_membership(
        &mut self,
        hash: Hash,
        block_creator: &PeerId,
        message_sender: &PeerId,
    ) -> bool {
        //We see this block first time
        if !self.broadcast_groups.contains(&hash) {
            //This can happen at startup for example when node is not ready yet(caught up with the network)
            if self.is_empty() {
                warn!(
                    "Received new block {:?} but current group is empty, rejecting the block",
                    hash
                );
                return false;
            }
        }

        //Make sure that the sender peer_id and block peer_id are part of the block initial group
        //1. If the block is new, the group is the current one
        //2. If the block is old, the group is the one that was used when the block was created

        //It's needed to make sure that
        //1. The peer is authenticated(part of the network)
        //2. Block processing is consistent regarding the group across rounds

        let membership_id = *self.broadcast_groups.get(&hash).unwrap_or(&self.current_id);

        //Node is excluded from group for some reason(for example health checks failed)
        if !self.is_member(membership_id, message_sender) {
            warn!(
                "Received new block {} but sender {} is not part of the current group",
                hash, message_sender
            );
            return false;
        }

        //Node is excluded from group for some reason(for example health checks failed)
        if !self.is_member(membership_id, block_creator) {
            warn!(
                "Received new block {} but sender {} is not part of the current group",
                hash, message_sender
            );
            return false;
        }

        self.broadcast_groups.put(hash, membership_id);

        true
    }

    pub(crate) fn get_group_by_block_hash(&mut self, hash: Hash) -> Option<&HashSet<PeerId>> {
        let membership_id = *self.broadcast_groups.get(&hash)?;
        self.snapshots.get(&membership_id)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::broadcast::group::BroadcastGroup;
    use crate::peer::PeerId;
    use crate::utilities::hash::Hash;

    #[test]
    fn test_no_snapshot() {
        let group = BroadcastGroup::new();
        assert_eq!(group.current_id, 0);
        //Including initial default snapshot
        assert_eq!(group.snapshots.len(), 1);
    }

    #[test]
    fn test_multiple_snapshots() {
        let (mut group, snapshots) = group_with_snapshots(10);
        assert_eq!(group.current_id, 10);
        //Including initial default snapshot
        assert_eq!(group.snapshots.len(), 11);

        for (i, sn) in snapshots
            .iter()
            .enumerate()
            .map(|(i, sn)| ((i + 1) as u64, sn))
        {
            let gsn = group.snapshots.get(&i).unwrap();
            assert_eq!(sn, gsn);
        }
    }

    #[test]
    fn check_membership_empty_group() {
        let mut group = BroadcastGroup::new();
        let hash = Hash::new([0; 32]);
        assert!(!group.check_membership(hash, &PeerId::random(), &PeerId::random()));
        assert!(!group.broadcast_groups.contains(&hash));
    }

    #[test]
    fn check_membership_creator_nor_sender_not_member() {
        let (mut group, _snapshots) = group_with_snapshots(1);
        assert!(!group.check_membership(Hash::new([0; 32]), &PeerId::random(), &PeerId::random()));
        assert!(!group.broadcast_groups.contains(&Hash::new([0; 32])));
    }

    #[test]
    fn check_membership_creator_not_member() {
        let (mut group, snapshots) = group_with_snapshots(1);
        let sender = snapshots[0].clone().into_iter().next().unwrap();

        let hash = Hash::new([0; 32]);
        assert!(!group.check_membership(hash, &PeerId::random(), &sender));
        assert!(!group.broadcast_groups.contains(&hash));
    }

    #[test]
    fn check_membership_sender_not_member() {
        let (mut group, snapshots) = group_with_snapshots(1);
        let creator = snapshots[0].clone().into_iter().next().unwrap();
        let hash = Hash::new([0; 32]);
        assert!(!group.check_membership(hash, &creator, &PeerId::random()));
        assert!(!group.broadcast_groups.contains(&hash));
    }

    #[test]
    fn check_snapshot_membership_both_are_members() {
        let (mut group, snapshots) = group_with_snapshots(1);
        let creator = snapshots[0].clone().into_iter().next().unwrap();
        let sender = creator;
        let hash = Hash::new([0; 32]);
        assert!(group.check_membership(hash, &creator, &sender));
        assert!(group.broadcast_groups.contains(&hash));
    }

    #[test]
    fn check_snapshot_membership_of_current_snapshot() {
        let (mut group, snapshots) = group_with_snapshots(2);
        let current_snapshot = snapshots[1].clone();
        let creator = current_snapshot.into_iter().next().unwrap();
        let sender = creator;

        let hash = Hash::new([0; 32]);
        assert!(group.check_membership(hash, &creator, &sender));
        assert!(group.broadcast_groups.contains(&hash));

        //Remove the current snapshot
        group.snapshots.pop(&group.current_id);

        //Membership should fail
        assert!(!group.check_membership(hash, &creator, &sender));
    }

    #[test]
    fn check_snapshot_membership_of_previous_snapshot() {
        let mut group = BroadcastGroup::new();
        let first_snapshot = create_snapshot();
        group.add_snapshot(first_snapshot.clone());

        let creator = first_snapshot.into_iter().next().unwrap();
        let sender = creator;
        let hash = Hash::new([0; 32]);
        assert!(group.check_membership(hash, &creator, &sender));
        assert!(group.broadcast_groups.contains(&hash));

        //Add second snapshot
        group.add_snapshot(create_snapshot());

        //Remove the current snapshot
        group.snapshots.pop(&group.current_id);

        //Membership should still pass
        assert!(group.broadcast_groups.contains(&hash));
        assert!(group.check_membership(hash, &creator, &sender));
    }

    fn group_with_snapshots(count: usize) -> (BroadcastGroup, Vec<HashSet<PeerId>>) {
        let mut group = BroadcastGroup::new();
        let mut snapshots = Vec::new();
        for _ in 0..count {
            let snapshot = create_snapshot();
            snapshots.push(snapshot.clone());
            group.add_snapshot(snapshot);
        }
        (group, snapshots)
    }

    fn create_snapshot() -> HashSet<PeerId> {
        let mut snapshot = HashSet::new();
        let peer_id = PeerId::random();
        snapshot.insert(peer_id);
        snapshot
    }
}
