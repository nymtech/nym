use super::Mixnode;

/// A (currently RAM-based) data store to keep tabs on which nodes have what
/// stake assigned to them.
#[derive(Clone, Debug, PartialEq)]
pub struct MixminingDb {
    pub mixnodes: Vec<Mixnode>,
    pub capacity: u32,
}

impl MixminingDb {
    pub fn new() -> MixminingDb {
        let mixnodes = Vec::<Mixnode>::new();
        MixminingDb {
            capacity: 0,
            mixnodes,
        }
    }
}
