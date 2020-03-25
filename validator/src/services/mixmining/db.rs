use super::Mixnode;

/// A (currently RAM-based) data store to keep tabs on which nodes have what
/// stake assigned to them.
#[derive(Clone, Debug, PartialEq)]
pub struct MixminingDb {
    pub mixnodes: Box<Vec<Mixnode>>,
}

impl MixminingDb {
    pub fn new() -> MixminingDb {
        let mut mixnodes = Vec::<Mixnode>::new();
        MixminingDb {
            mixnodes: Box::new(mixnodes),
        }
    }
}
