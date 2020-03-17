use super::staking::Mixnode;

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
