use serde::{Deserialize, Serialize};

use crate::utilities::hash::{EphemeraHasher, Hash, Hasher};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Serialize, Deserialize)]
pub struct MerkleTree {
    leaf_count: usize,
    nodes: Vec<Hash>,
}

impl MerkleTree {
    pub(crate) fn build_tree(leaves: &[Hash]) -> Self {
        if leaves.is_empty() {
            return Self {
                leaf_count: 1,
                //So every message which matches this hash will be accepted
                //Not sure if it's a problem
                nodes: vec![Hash::new([0; 32])],
            };
        }

        let leaf_count = leaves.len();
        let mut nodes = Vec::with_capacity(leaf_count * 2);
        nodes.extend_from_slice(leaves);

        let mut prev_level_len = leaf_count;
        let mut prev_offset = 0;
        let mut current_offset = leaf_count;

        while prev_level_len > 1 {
            let current_level_len = (prev_level_len + 1) / 2;

            for i in 0..current_level_len {
                let prev_index = i * 2;
                let left = nodes[prev_offset + prev_index];
                let right = if prev_index + 1 < prev_level_len {
                    nodes[prev_offset + prev_index + 1]
                } else {
                    nodes[prev_offset + prev_index]
                };
                let hash = Hasher::digest(&[left.inner(), right.inner()].concat()).into();
                nodes.push(hash);
            }
            prev_level_len = current_level_len;
            prev_offset = current_offset;
            current_offset += current_level_len;
        }
        Self { leaf_count, nodes }
    }

    pub(crate) fn root_hash(&self) -> Hash {
        self.nodes[self.nodes.len() - 1]
    }

    pub(crate) fn verify_leaf_at_index(&self, hash: Hash, leaf_index: usize) -> bool {
        let mut level_offset = 0;
        let mut level_len = self.leaf_count;
        let mut leaf_index = leaf_index;
        let mut current_hash = hash;
        while level_offset + level_len < self.nodes.len() {
            let level = &self.nodes[level_offset..(level_offset + level_len)];
            if leaf_index % 2 == 0 {
                let right_index = if leaf_index + 1 < level_len {
                    leaf_index + 1
                } else {
                    leaf_index
                };
                current_hash =
                    Hasher::digest(&[current_hash.inner(), level[right_index].inner()].concat())
                        .into();
            } else {
                current_hash =
                    Hasher::digest(&[level[leaf_index - 1].inner(), current_hash.inner()].concat())
                        .into();
            }
            leaf_index /= 2;
            level_offset += level_len;
            level_len = (level_len + 1) / 2;
        }
        current_hash == self.root_hash()
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use rand::RngCore;

    use crate::utilities::hash::Hash;

    use super::*;

    #[test]
    fn test_merkle() {
        //Technically testing one implementation against another...
        for i in 1..10 {
            let mut rnd = rand::thread_rng();
            let leaves = iter::repeat_with(|| {
                let mut bytes = [0u8; 32];
                rnd.fill_bytes(&mut bytes);
                Hash::new(bytes)
            })
            .take(i)
            .collect::<Vec<_>>();

            let mut level: Vec<Hash> = leaves.clone();

            while level.len() > 1 {
                level = level
                    .chunks(2)
                    .map(|chunk| {
                        if chunk.len() == 1 {
                            Hasher::digest(&[chunk[0].inner(), chunk[0].inner()].concat()).into()
                        } else {
                            Hasher::digest(&[chunk[0].inner(), chunk[1].inner()].concat()).into()
                        }
                    })
                    .collect();
            }

            let root: Hash = level[0];

            let tree = MerkleTree::build_tree(&leaves);
            assert_eq!(tree.root_hash(), root);
        }
    }

    #[test]
    fn test_verify_leaf() {
        for i in 0..10 {
            let mut rnd = rand::thread_rng();
            let right_leaves = iter::repeat_with(|| {
                let mut bytes = [0u8; 32];
                rnd.fill_bytes(&mut bytes);
                Hash::new(bytes)
            })
            .take(i)
            .collect::<Vec<_>>();

            let wrong_leaves = iter::repeat_with(|| {
                let mut bytes = [0u8; 32];
                rnd.fill_bytes(&mut bytes);
                Hash::new(bytes)
            })
            .take(i)
            .collect::<Vec<_>>();

            let tree = MerkleTree::build_tree(&right_leaves);

            for (i, (correct, wrong)) in right_leaves
                .into_iter()
                .zip(wrong_leaves.into_iter())
                .enumerate()
            {
                assert!(tree.verify_leaf_at_index(correct, i));
                assert!(!tree.verify_leaf_at_index(wrong, i));
            }
        }
    }
}
