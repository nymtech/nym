// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::proof_discrete_log::ProofOfDiscreteLog;
use crate::bte::{Epoch, Params, Tau};
use crate::error::DkgError;
use crate::utils::{deserialize_g1, deserialize_g2, deserialize_scalar};
use bls12_381::{G1Projective, G2Projective, Scalar};
use ff::Field;
use group::GroupEncoding;
use rand_core::RngCore;
use zeroize::Zeroize;

#[derive(Debug, Zeroize)]
#[zeroize(drop)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub(crate) struct Node {
    pub(crate) tau: Tau,

    // g1^rho
    pub(crate) a: G1Projective,

    // g2^x
    pub(crate) b: G2Projective,

    // f_i^rho, up to lambda_t elements
    pub(crate) ds: Vec<G2Projective>,

    // fh_i^rho, always lambda_h elements
    pub(crate) dh: Vec<G2Projective>,

    // h^rho
    pub(crate) e: G2Projective,
}

impl Node {
    fn new_root(
        a: G1Projective,
        b: G2Projective,
        ds: Vec<G2Projective>,
        dh: Vec<G2Projective>,
        e: G2Projective,
    ) -> Self {
        Node {
            tau: Tau::new_root(),
            a,
            b,
            ds,
            dh,
            e,
        }
    }

    fn is_root(&self) -> bool {
        self.tau.0.is_empty()
    }

    pub(crate) fn reblind(&mut self, params: &Params, mut rng: impl RngCore) {
        let delta = Scalar::random(&mut rng);
        self.a += G1Projective::generator() * delta;

        // TODO: or do we have to do full tau evaluation here?
        self.b += self.tau.evaluate_partial_f(params) * delta;
        self.ds
            .iter_mut()
            .zip(params.fs.iter().skip(self.tau.height()))
            .for_each(|(d_i, f_i)| *d_i += f_i * delta);
        self.dh
            .iter_mut()
            .zip(params.fh.iter())
            .for_each(|(d_i, f_i)| *d_i += f_i * delta);

        self.e += params.h * delta;
    }

    // note: it's unsafe to use this method outside `try_update_to` as
    // we have guaranteed there that `self` is parent of the target
    // and that `self.tau != target_tau`
    /// Given `self` with `Tau1` and `target_tau` with `Tau2`, such that `Tau1` prefixes `Tau2`,
    /// i.e. `Tau2 == Tau1 || SUFFIX`, and `Tau2` is a leaf node, derive all required crypto material
    /// for its construction.
    fn derive_target_child_with_partials(
        &self,
        params: &Params,
        target_tau: Tau,
        partial_b: &G2Projective,
        partial_f: &G2Projective,
        mut rng: impl RngCore,
    ) -> Self {
        debug_assert!(self.tau.is_parent_of(&target_tau));
        debug_assert_ne!(self.tau, target_tau);

        let delta = Scalar::random(&mut rng);
        let a = self.a + G1Projective::generator() * delta;
        let b = partial_b + partial_f * delta;
        let ds = self
            .ds
            .iter()
            .zip(params.fs.iter())
            .skip(target_tau.height())
            .map(|(d_i, f_i)| d_i + f_i * delta)
            .collect();
        let dh = self
            .dh
            .iter()
            .zip(params.fh.iter())
            .map(|(dh_i, fh_i)| dh_i + fh_i * delta)
            .collect();
        let e = self.e + params.h * delta;

        Node {
            tau: target_tau,
            a,
            b,
            ds,
            dh,
            e,
        }
    }

    // note: it's unsafe to use this method outside `try_update_to` as
    // we have guaranteed there that `self` is parent of the target
    // and that `self.tau != target_tau`
    /// Given `self` with `Tau1` and `most_direct_parent` with `Tau2`, such that `Tau1` prefixes `Tau2`,
    /// i.e. `Tau2 == Tau1 || SUFFIX`, derive node with `Tau3 = Tau2 || 1`
    fn derive_right_nonfinal_child_of_with_partials(
        &self,
        params: &Params,
        most_direct_parent: Tau,
        partial_b: &G2Projective,
        partial_f: &G2Projective,
        mut rng: impl RngCore,
    ) -> Self {
        let right_branch = most_direct_parent.right_child();

        debug_assert!(self.tau.is_parent_of(&most_direct_parent));
        debug_assert!(self.tau.is_parent_of(&right_branch));
        debug_assert_ne!(self.tau, right_branch);

        // n is height difference between self and the child
        let n = right_branch.height() - self.tau.height();

        // i is the index of the last bit we just added
        let i = right_branch.height() - 1;

        let delta = Scalar::random(&mut rng);
        let a = self.a + G1Projective::generator() * delta;
        let d0 = self.ds[n - 1];
        let b = partial_b + d0 + (partial_f + params.fs[i]) * delta;
        let ds = self
            .ds
            .iter()
            .skip(n)
            .zip(params.fs.iter().skip(right_branch.height()))
            .map(|(d_i, f_i)| d_i + f_i * delta)
            .collect();
        let dh = self
            .dh
            .iter()
            .zip(params.fh.iter())
            .map(|(dh_i, fh_i)| dh_i + fh_i * delta)
            .collect();

        let e = self.e + params.h * delta;

        Node {
            tau: right_branch,
            a,
            b,
            ds,
            dh,
            e,
        }
    }

    // tau_bytes_len || tau || a || b || len_ds || ds || len_dh || dh || e
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let g1_elements = 1;
        let g2_elements = self.ds.len() + self.dh.len() + 2;

        let tau_bytes = self.tau.to_bytes();

        // the extra 12 comes from the triple u32 we use for encoding lengths of tau, ds and dh
        let mut bytes =
            Vec::with_capacity(tau_bytes.len() + g1_elements * 48 + g2_elements * 96 + 12);

        bytes.extend_from_slice(&((tau_bytes.len() as u32).to_be_bytes()));
        bytes.extend_from_slice(&tau_bytes);
        bytes.extend_from_slice(self.a.to_bytes().as_ref());
        bytes.extend_from_slice(self.b.to_bytes().as_ref());
        bytes.extend_from_slice(&((self.ds.len() as u32).to_be_bytes()));
        for d_i in &self.ds {
            bytes.extend_from_slice(d_i.to_bytes().as_ref());
        }
        bytes.extend_from_slice(&((self.dh.len() as u32).to_be_bytes()));
        for dh_i in &self.dh {
            bytes.extend_from_slice(dh_i.to_bytes().as_ref());
        }
        bytes.extend_from_slice(self.e.to_bytes().as_ref());

        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        // at the very least we require bytes for:
        // - tau_len ( 4 )
        // - tau ( could be 0 for root node )
        // - a ( 48 )
        // - b ( 96 )
        // - length indication of ds ( 4 )
        // - length indication of dh ( 4 )
        // - e ( 96 )
        if bytes.len() < 4 + 48 + 96 + 4 + 4 + 96 {
            return Err(DkgError::new_deserialization_failure(
                "Node",
                "insufficient number of bytes provided",
            ));
        }

        let tau_len = u32::from_be_bytes((&bytes[..4]).try_into().unwrap()) as usize;
        let mut i = 4;

        let tau = Tau::try_from_bytes(&bytes[i..i + tau_len])?;
        i += tau_len;

        // perform another length check to account for bytes consumed by tau
        if bytes[i..].len() < 48 + 96 + 4 + 4 + 96 {
            return Err(DkgError::new_deserialization_failure(
                "Node",
                "insufficient number of bytes provided",
            ));
        }

        let a = deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
            DkgError::new_deserialization_failure("Node.a", "invalid curve point")
        })?;
        i += 48;

        let b = deserialize_g2(&bytes[i..i + 96]).ok_or_else(|| {
            DkgError::new_deserialization_failure("Node.b", "invalid curve point")
        })?;
        i += 96;

        let ds_len = u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        if bytes[i..].len() < ds_len * 96 + 4 {
            return Err(DkgError::new_deserialization_failure(
                "Node",
                "insufficient number of bytes provided (ds)",
            ));
        }

        let mut ds = Vec::with_capacity(ds_len);
        for j in 0..ds_len {
            let d_i = deserialize_g2(&bytes[i..i + 96]).ok_or_else(|| {
                DkgError::new_deserialization_failure(
                    format!("Node.ds_{}", j),
                    "invalid curve point",
                )
            })?;

            ds.push(d_i);
            i += 96;
        }

        let dh_len = u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        if bytes[i..].len() != (dh_len + 1) * 96 {
            return Err(DkgError::new_deserialization_failure(
                "Node",
                "insufficient number of bytes provided (dh)",
            ));
        }

        let mut dh = Vec::with_capacity(dh_len);
        for j in 0..dh_len {
            let dh_i = deserialize_g2(&bytes[i..i + 96]).ok_or_else(|| {
                DkgError::new_deserialization_failure(
                    format!("Node.dh_{}", j),
                    "invalid curve point",
                )
            })?;

            dh.push(dh_i);
            i += 96;
        }

        let e = deserialize_g2(&bytes[i..]).ok_or_else(|| {
            DkgError::new_deserialization_failure("Node.h", "invalid curve point")
        })?;

        Ok(Node {
            tau,
            a,
            b,
            ds,
            dh,
            e,
        })
    }
}

// produces public key and a decryption key for the root of the tree
pub fn keygen(params: &Params, mut rng: impl RngCore) -> (DecryptionKey, PublicKeyWithProof) {
    let g1 = G1Projective::generator();
    let g2 = G2Projective::generator();

    let mut x = Scalar::random(&mut rng);
    let y = g1 * x;

    let proof = ProofOfDiscreteLog::construct(&mut rng, &y, &x);

    let mut rho = Scalar::random(&mut rng);

    let a = g1 * rho;
    let b = g2 * x + params.f0 * rho;

    let ds = params.fs.iter().map(|f_i| f_i * rho).collect();
    let dh = params.fh.iter().map(|fh_i| fh_i * rho).collect();
    let e = params.h * rho;

    let dk = DecryptionKey::new_root(Node::new_root(a, b, ds, dh, e));

    let public_key = PublicKey(y);
    let key_with_proof = PublicKeyWithProof {
        key: public_key,
        proof,
    };

    x.zeroize();
    rho.zeroize();

    (dk, key_with_proof)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PublicKey(pub(crate) G1Projective);

impl PublicKey {
    pub fn verify(&self, proof: &ProofOfDiscreteLog) -> bool {
        proof.verify(&self.0)
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct PublicKeyWithProof {
    pub(crate) key: PublicKey,
    pub(crate) proof: ProofOfDiscreteLog,
}

impl PublicKeyWithProof {
    pub fn verify(&self) -> bool {
        self.key.verify(&self.proof)
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // we have 2 G1 elements and 1 Scalar
        let mut bytes = Vec::with_capacity(2 * 48 + 32);
        bytes.extend_from_slice(self.key.0.to_bytes().as_ref());
        bytes.extend_from_slice(self.proof.rand_commitment.to_bytes().as_ref());
        bytes.extend_from_slice(self.proof.response.to_bytes().as_ref());

        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        if bytes.len() != 2 * 48 + 32 {
            return Err(DkgError::new_deserialization_failure(
                "PublicKeyWithProof",
                "provided bytes had invalid length",
            ));
        }

        let y_bytes = &bytes[..48];
        let commitment_bytes = &bytes[48..96];
        let response_bytes = &bytes[96..];

        let y = deserialize_g1(y_bytes).ok_or_else(|| {
            DkgError::new_deserialization_failure("PublicKeyWithProof.key.0", "invalid curve point")
        })?;

        let rand_commitment = deserialize_g1(commitment_bytes).ok_or_else(|| {
            DkgError::new_deserialization_failure(
                "PublicKeyWithProof.proof.rand_commitment",
                "invalid curve point",
            )
        })?;

        let response = deserialize_scalar(response_bytes).ok_or_else(|| {
            DkgError::new_deserialization_failure(
                "PublicKeyWithProof.proof.response",
                "invalid scalar",
            )
        })?;

        Ok(PublicKeyWithProof {
            key: PublicKey(y),
            proof: ProofOfDiscreteLog {
                rand_commitment,
                response,
            },
        })
    }
}

#[derive(Debug, Zeroize)]
#[zeroize(drop)]
#[cfg_attr(test, derive(PartialEq))]
pub struct DecryptionKey {
    // note that the nodes are ordered from "right" to "left"
    pub(crate) nodes: Vec<Node>,
}

impl DecryptionKey {
    fn new_root(root_node: Node) -> Self {
        DecryptionKey {
            nodes: vec![root_node],
        }
    }

    fn current(&self) -> Result<&Node, DkgError> {
        // we must have at least a single node, otherwise we have a malformed key
        self.nodes.last().ok_or(DkgError::MalformedDecryptionKey)
    }

    pub fn current_epoch(&self, params: &Params) -> Result<Option<Epoch>, DkgError> {
        let current_node = self.current()?;
        if current_node.is_root() {
            Ok(None)
        } else {
            Epoch::try_from_tau(&current_node.tau, params).map(Option::Some)
        }
    }

    pub(crate) fn try_get_compatible_node(&self, epoch: Epoch) -> Result<&Node, DkgError> {
        let tau = epoch.as_tau();
        self.nodes
            .iter()
            .rev()
            .find(|node| node.tau.is_parent_of(&tau))
            .ok_or(DkgError::ExpiredKey)
    }

    pub fn try_update_to_next_epoch(
        &mut self,
        params: &Params,
        mut rng: impl RngCore,
    ) -> Result<(), DkgError> {
        if self.nodes.is_empty() {
            return Err(DkgError::MalformedDecryptionKey);
        }

        let mut target_epoch = Epoch::new(0);
        if self.nodes.len() == 1 && self.nodes[0].is_root() {
            return self.try_update_to(target_epoch, params, &mut rng);
        }

        // unwrap is fine as we have asserted self.nodes is not empty
        self.nodes.pop().unwrap();

        if let Some(tail) = self.nodes.last() {
            target_epoch = tail.tau.lowest_valid_epoch_child(params)?;
        } else {
            // essentially our key consisted of only a single node and it wasn't a root,
            // so either it was malformed or we somehow reached the final epoch and wanted to update
            // beyond that. Either way, update to l + 1 is impossible
            return Err(DkgError::MalformedDecryptionKey);
        }

        self.try_update_to(target_epoch, params, &mut rng)
    }

    /// Attempts to update `self` to the provided `epoch`. If the update is not possible,
    /// because the target was in the past or the key is malformed, an error is returned.
    ///
    /// Note that this method mutates the key in place and if the original key was malformed,
    /// there are no guarantees about its internal state post-call.
    pub fn try_update_to(
        &mut self,
        target_epoch: Epoch,
        params: &Params,
        mut rng: impl RngCore,
    ) -> Result<(), DkgError> {
        if self.nodes.is_empty() {
            // somehow we have an empty decryption key
            return Err(DkgError::MalformedDecryptionKey);
        }

        // makes it easier to work with since we will be generating non-leaf nodes
        let target_tau = target_epoch.as_tau();
        let current_tau = &self.current()?.tau;

        if current_tau == &target_tau {
            // our key is already updated to the target
            return Ok(());
        }

        if current_tau > &target_tau {
            // we cannot derive keys for past epochs
            return Err(DkgError::TargetEpochUpdateInThePast);
        }

        // drop the nodes that are no longer required and get the most direct parent for the target epoch available
        let mut parent = loop {
            // if pop() fails the key is malformed since we checked that the target_epoch > current_epoch,
            // hence the update should have been possible
            let tail = self.nodes.pop().ok_or(DkgError::MalformedDecryptionKey)?;
            if tail.tau.is_parent_of(&target_tau) {
                break tail;
            }
        };

        // essentially the case of updating epoch n to n + 1, where n is even;
        // in that case the last two nodes are [..., epoch_{n+1}, epoch_n]
        // so we just have to reblind the n+1 node and we're done
        if parent.tau == target_tau {
            parent.reblind(params, &mut rng);
            self.nodes.push(parent);
            return Ok(());
        }

        // accumulators, note that the previous elements have already been included by the parent,
        // i.e. for example for parent at height l <= n, b = g2^x * f0^rho * d1^{tau_1} * ... * dl^{tau_l}
        // new_b_accumulator = b * d1^{tau_1} * d2^{tau_2} * ... * dn^{tau_n}
        // new_f_accumulator = f0 * f1^{tau_1} * f2^{tau_2} * ... * fn^{tau_n} (up to lambda_t)
        let mut new_b_accumulator = parent.b;
        let mut new_f_accumulator = parent.tau.evaluate_partial_f(params);

        let parent_height = parent.tau.height();

        // path from the parent to the child
        for (n, bit) in target_tau
            .0
            .iter()
            .by_vals()
            .skip(parent.tau.height())
            .enumerate()
        {
            // ith bit of the [child] epoch
            // note that n represents height difference between parent and the current bit
            let i = n + parent_height;

            // if the bit is NOT set, push the right '1' subtree (for future keys)
            // so for example if given parent with some `PREFIX` tau and target_epoch being `PREFIX || 010`,
            // in the first loop iteration we're going to look at bit `0` and
            // derive child node `PREFIX || 1` so that in the future we could derive keys for all other epochs starting with `PREFIX || 1`
            // in the next loop iteration we're going to look at bit `1` and simply update the accumulators,
            // as we don't need to generate any "left" nodes as all of them would have constructed epochs that are already in the past
            // finally, in the last iteration, we look at the bit `0` and derive node `PREFIX || 011`,
            // i.e. the one that FOLLOWS the target node.
            if !bit {
                let direct_parent = target_tau.try_get_parent_at_height(i)?;

                self.nodes
                    .push(parent.derive_right_nonfinal_child_of_with_partials(
                        params,
                        direct_parent,
                        &new_b_accumulator,
                        &new_f_accumulator,
                        &mut rng,
                    ));
            } else {
                // only update the accumulators when the bit is set, as d^0 == identity, so there's
                // no point in doing anything else;
                // note that we don't have to generate any new nodes when going into the right branch
                // of the tree as everything on the left would have been in the past, so we don't care about them
                new_b_accumulator += parent.ds[n]; // add d0
                new_f_accumulator += params.fs[i]; // f_i
            }
        }

        self.nodes.push(parent.derive_target_child_with_partials(
            params,
            target_epoch.as_tau(),
            &new_b_accumulator,
            &new_f_accumulator,
            &mut rng,
        ));

        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let num_nodes = self.nodes.len() as u32;

        // unfortunately we're not going to know the expected capacity
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&num_nodes.to_be_bytes());

        for node in &self.nodes {
            let mut node_bytes = node.to_bytes();
            bytes.extend_from_slice(&((node_bytes.len() as u32).to_be_bytes()));
            bytes.append(&mut node_bytes)
        }

        bytes
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Self, DkgError> {
        // we have to be able to read the length of nodes
        if b.len() < 4 {
            return Err(DkgError::new_deserialization_failure(
                "DecryptionKey",
                "insufficient number of bytes provided",
            ));
        }
        let nodes_len = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;
        let mut nodes = Vec::with_capacity(nodes_len);

        let mut i = 4;
        for _ in 0..nodes_len {
            // check if we can actually read the length...
            if b[i..].len() < 4 {
                return Err(DkgError::new_deserialization_failure(
                    "DecryptionKey.Node",
                    "insufficient number of bytes provided for BTE Node recovery",
                ));
            }

            let node_bytes = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
            if b[i + 4..].len() < node_bytes {
                return Err(DkgError::new_deserialization_failure(
                    "DecryptionKey.Node",
                    "insufficient number of bytes provided for BTE Node recovery",
                ));
            }
            i += 4;

            let node = Node::try_from_bytes(&b[i..i + node_bytes])?;
            nodes.push(node);
            i += node_bytes;
        }

        Ok(DecryptionKey { nodes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bte::setup;
    use bitvec::bitvec;
    use bitvec::order::Msb0;
    use rand_core::SeedableRng;

    #[test]
    fn basic_coverage_nodes() {
        // it's some basic test I've been performing when writing the update function, but figured
        // might as well put it into a unit test. note that it doesn't check the entire structure,
        // but just the few last nodes of low height

        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (mut dk, _) = keygen(&params, &mut rng);

        let root_node_copy = dk.nodes.clone();

        // this is a root node
        assert_eq!(dk.nodes.len(), 1);
        assert!(dk.nodes[0].is_root());

        // we have to have a node for right branch on each height (1, 01, 001, ... etc)
        // plus an additional one for the two left-most leaves (epochs "0" and "1")
        dk.try_update_to(Epoch::new(0), &params, &mut rng).unwrap();
        assert_eq!(dk.nodes.len(), 33);

        let expected_last = Tau::new(0);
        // (and yes, I had to look up those names in a thesaurus)
        let expected_penultimate = Tau::new(1);
        // note that this value is 31bit long
        let expected_antepenultimate = Tau(bitvec![u32, Msb0;
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1
        ]);

        let mut nodes_iter = dk.nodes.iter().rev();
        assert_eq!(expected_last, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_penultimate, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_antepenultimate, nodes_iter.next().unwrap().tau);

        let mut epoch_zero_nodes = dk.nodes.clone();

        // nodes for epoch1 should be identical for those for epoch0 minus the 00..00 leaf
        dk.try_update_to(Epoch::new(1), &params, &mut rng).unwrap();
        assert_eq!(dk.nodes.len(), 32);
        epoch_zero_nodes.pop().unwrap();
        assert_eq!(
            epoch_zero_nodes
                .iter()
                .map(|node| node.tau.clone())
                .collect::<Vec<_>>(),
            dk.nodes
                .iter()
                .map(|node| node.tau.clone())
                .collect::<Vec<_>>()
        );

        dk.try_update_to(Epoch::new(2), &params, &mut rng).unwrap();
        dk.try_update_to(Epoch::new(3), &params, &mut rng).unwrap();
        dk.try_update_to(Epoch::new(4), &params, &mut rng).unwrap();

        let expected_last = Tau::new(4);
        let expected_penultimate = Tau::new(5);
        let expected_antepenultimate = Tau(bitvec![u32, Msb0;
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1
        ]);
        let expected_preantepenultimate = Tau(bitvec![u32, Msb0;
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1
        ]);
        assert_eq!(dk.nodes.len(), 32);
        let mut nodes_iter = dk.nodes.iter().rev();
        assert_eq!(expected_last, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_penultimate, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_antepenultimate, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_preantepenultimate, nodes_iter.next().unwrap().tau);

        // the result should be the same of regardless if we update incrementally or go to the target immediately
        let mut new_root = DecryptionKey {
            nodes: root_node_copy,
        };
        new_root
            .try_update_to(Epoch::new(4), &params, &mut rng)
            .unwrap();
        assert_eq!(
            dk.nodes
                .iter()
                .map(|node| node.tau.clone())
                .collect::<Vec<_>>(),
            new_root
                .nodes
                .iter()
                .map(|node| node.tau.clone())
                .collect::<Vec<_>>()
        );

        // getting expected nodes for those epochs is non-trivial for test purposes, but the last node
        // should ALWAYS be equal to the target epoch
        dk.try_update_to(Epoch::new(42), &params, &mut rng).unwrap();
        assert_eq!(dk.nodes.last().unwrap().tau, Tau::new(42));
        dk.try_update_to(Epoch::new(123456), &params, &mut rng)
            .unwrap();
        assert_eq!(dk.nodes.last().unwrap().tau, Tau::new(123456));
        dk.try_update_to(Epoch::new(3292547435), &params, &mut rng)
            .unwrap();
        assert_eq!(dk.nodes.last().unwrap().tau, Tau::new(3292547435));

        // trying to go to past epochs fails
        assert!(dk
            .try_update_to(Epoch::new(531), &params, &mut rng)
            .is_err())
    }

    #[test]
    fn updating_to_next_epoch() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (mut dk, _) = keygen(&params, &mut rng);

        // for root node current epoch is `None`
        assert_eq!(None, dk.current_epoch(&params).unwrap());

        // for root node it should result in epoch 0
        dk.try_update_to_next_epoch(&params, &mut rng).unwrap();
        assert_eq!(Some(Epoch::new(0)), dk.current_epoch(&params).unwrap());

        dk.try_update_to_next_epoch(&params, &mut rng).unwrap();
        assert_eq!(Some(Epoch::new(1)), dk.current_epoch(&params).unwrap());

        dk.try_update_to_next_epoch(&params, &mut rng).unwrap();
        assert_eq!(Some(Epoch::new(2)), dk.current_epoch(&params).unwrap());

        // if we start from some non-root epoch, it should result in l + 1
        dk.try_update_to(Epoch::new(42), &params, &mut rng).unwrap();
        dk.try_update_to_next_epoch(&params, &mut rng).unwrap();
        assert_eq!(Some(Epoch::new(43)), dk.current_epoch(&params).unwrap());

        dk.try_update_to(Epoch::new(12345), &params, &mut rng)
            .unwrap();
        dk.try_update_to_next_epoch(&params, &mut rng).unwrap();
        assert_eq!(Some(Epoch::new(12346)), dk.current_epoch(&params).unwrap());

        dk.try_update_to(Epoch::new(3292547435), &params, &mut rng)
            .unwrap();
        dk.try_update_to_next_epoch(&params, &mut rng).unwrap();
        assert_eq!(
            Some(Epoch::new(3292547436)),
            dk.current_epoch(&params).unwrap()
        );
    }

    #[test]
    fn public_key_with_proof_roundtrip() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (_, pk) = keygen(&params, &mut rng);
        let bytes = pk.to_bytes();
        let recovered = PublicKeyWithProof::try_from_bytes(&bytes).unwrap();

        assert_eq!(pk, recovered)
    }

    #[test]
    fn bte_node_roundtrip() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (mut dk, _) = keygen(&params, &mut rng);

        let root_node = dk.nodes[0].clone();
        let bytes = root_node.to_bytes();
        let recovered = Node::try_from_bytes(&bytes).unwrap();
        assert_eq!(root_node, recovered);

        dk.try_update_to(Epoch::new(3292547435), &params, &mut rng)
            .unwrap();
        for node in &dk.nodes {
            let bytes = node.to_bytes();
            let recovered = Node::try_from_bytes(&bytes).unwrap();
            assert_eq!(node, &recovered);
        }
    }

    #[test]
    fn decryption_key_node_roundtrip() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (mut dk, _) = keygen(&params, &mut rng);

        let bytes = dk.to_bytes();
        let recovered = DecryptionKey::try_from_bytes(&bytes).unwrap();
        assert_eq!(dk, recovered);

        dk.try_update_to(Epoch::new(0), &params, &mut rng).unwrap();
        let bytes = dk.to_bytes();
        let recovered = DecryptionKey::try_from_bytes(&bytes).unwrap();
        assert_eq!(dk, recovered);

        dk.try_update_to(Epoch::new(1), &params, &mut rng).unwrap();
        let bytes = dk.to_bytes();
        let recovered = DecryptionKey::try_from_bytes(&bytes).unwrap();
        assert_eq!(dk, recovered);

        dk.try_update_to(Epoch::new(42), &params, &mut rng).unwrap();
        let bytes = dk.to_bytes();
        let recovered = DecryptionKey::try_from_bytes(&bytes).unwrap();
        assert_eq!(dk, recovered);

        dk.try_update_to(Epoch::new(3292547435), &params, &mut rng)
            .unwrap();
        let bytes = dk.to_bytes();
        let recovered = DecryptionKey::try_from_bytes(&bytes).unwrap();
        assert_eq!(dk, recovered);
    }
}
