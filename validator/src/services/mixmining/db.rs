use super::Mixnode;

/// A (currently RAM-based) data store to keep tabs on which nodes have what
/// stake assigned to them.
#[derive(Clone, Debug, PartialEq)]
pub struct MixminingDb {
    mixnodes: Vec<Mixnode>,
    capacity: usize,
}

impl MixminingDb {
    pub fn new() -> MixminingDb {
        let mixnodes = Vec::<Mixnode>::new();
        MixminingDb {
            capacity: 6,
            mixnodes,
        }
    }

    pub fn add(&mut self, mixnode: Mixnode) {
        self.mixnodes.push(mixnode);
    }

    pub fn get_mixnodes(&self) -> &Vec<Mixnode> {
        &self.mixnodes
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod capacity {
    use super::*;

    #[test]
    fn starts_at_6() {
        let db = MixminingDb::new();
        assert_eq!(6, db.capacity());
    }

    #[test]
    fn setting_and_getting_work() {
        let mut db = MixminingDb::new();
        db.set_capacity(1);
        assert_eq!(1, db.capacity());
    }
}

#[cfg(test)]
mod adding_and_retrieving_mixnodes {
    use super::*;

    #[test]
    fn add_and_retrieve_one_works() {
        let node = fake_mixnode("London, UK");
        let mut db = MixminingDb::new();

        db.add(node.clone());

        assert_eq!(&node, db.get_mixnodes().first().unwrap());
    }

    #[test]
    fn add_and_retrieve_two_works() {
        let node1 = fake_mixnode("London, UK");
        let node2 = fake_mixnode("Neuchatel");
        let mut db = MixminingDb::new();

        db.add(node1.clone());
        db.add(node2.clone());

        assert_eq!(node1, db.get_mixnodes()[0]);
        assert_eq!(node2, db.get_mixnodes()[1]);
    }

    #[test]
    fn starts_empty() {
        let db = MixminingDb::new();
        assert_eq!(0, db.mixnodes.len());
    }

    #[test]
    fn calling_list_when_empty_returns_empty_vec() {
        let db = MixminingDb::new();
        let empty: Vec<Mixnode> = vec![];
        assert_eq!(&empty, db.get_mixnodes());
    }

    fn fake_mixnode(location: &str) -> Mixnode {
        Mixnode {
            host: String::from("foo.com"),
            last_seen: 123,
            location: String::from(location),
            public_key: String::from("abc123"),
            stake: 8,
            version: String::from("1.0"),
        }
    }
}
