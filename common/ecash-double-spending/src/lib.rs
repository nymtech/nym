// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bit_vec::BitVec;
use bloomfilter::Bloom;
use nym_network_defaults::{BloomfilterParameters, ECASH_DS_BLOOMFILTER_PARAMS};

pub struct DoubleSpendingFilter {
    params: BloomfilterParameters,
    inner: Bloom<Vec<u8>>,
}

impl Default for DoubleSpendingFilter {
    fn default() -> Self {
        DoubleSpendingFilter::new_empty_ecash()
    }
}

pub fn bloom_from_params<T>(params: &BloomfilterParameters, bitvec: BitVec) -> Bloom<Vec<T>> {
    assert_eq!(params.bitmap_size, bitvec.len() as u64);

    Bloom::from_bit_vec(
        bitvec,
        params.bitmap_size,
        params.num_hashes,
        params.sip_keys,
    )
}

impl DoubleSpendingFilter {
    pub fn new_empty(params: BloomfilterParameters) -> Self {
        let bitvec = BitVec::from_elem(params.bitmap_size as usize, false);
        DoubleSpendingFilter {
            inner: bloom_from_params(&params, bitvec),
            params,
        }
    }

    pub fn params(&self) -> BloomfilterParameters {
        self.params
    }

    pub fn rebuild(&self) -> DoubleSpendingFilterBuilder {
        DoubleSpendingFilterBuilder::new(self.params)
    }

    pub fn reset(&mut self) {
        self.inner.clear()
    }

    pub fn new_empty_ecash() -> Self {
        DoubleSpendingFilter::new_empty(ECASH_DS_BLOOMFILTER_PARAMS)
    }

    pub fn builder(params: BloomfilterParameters) -> DoubleSpendingFilterBuilder {
        DoubleSpendingFilterBuilder::new(params)
    }

    pub fn from_bytes(params: BloomfilterParameters, bitmap: &[u8]) -> Self {
        DoubleSpendingFilter {
            inner: bloom_from_params(&params, BitVec::from_bytes(bitmap)),
            params,
        }
    }

    pub fn replace_bitvec(&mut self, new: BitVec) {
        self.inner = bloom_from_params(&self.params, new)
    }

    pub fn dump_bitmap(&self) -> Vec<u8> {
        self.inner.bitmap()
    }

    pub fn set(&mut self, b: &Vec<u8>) {
        self.inner.set(b);
    }

    pub fn check(&self, b: &Vec<u8>) -> bool {
        self.inner.check(b)
    }
}

pub struct DoubleSpendingFilterBuilder {
    params: BloomfilterParameters,
    bit_vec_builder: Option<BitVecBuilder>,
}

impl DoubleSpendingFilterBuilder {
    pub fn new(params: BloomfilterParameters) -> Self {
        DoubleSpendingFilterBuilder {
            params,
            bit_vec_builder: None,
        }
    }

    pub fn add_bytes(&mut self, b: &[u8]) -> bool {
        match &mut self.bit_vec_builder {
            None => {
                self.bit_vec_builder = Some(BitVecBuilder::new(b));
                true
            }
            Some(builder) => builder.add_bytes(b),
        }
    }

    pub fn build(self) -> DoubleSpendingFilter {
        match self.bit_vec_builder {
            None => DoubleSpendingFilter::new_empty(self.params),
            Some(builder) => DoubleSpendingFilter {
                inner: bloom_from_params(&self.params, builder.finish()),
                params: self.params,
            },
        }
    }
}

pub struct BitVecBuilder(BitVec);

impl BitVecBuilder {
    pub fn new(initial_bitmap: &[u8]) -> Self {
        BitVecBuilder(BitVec::from_bytes(initial_bitmap))
    }

    pub fn add_bytes(&mut self, b: &[u8]) -> bool {
        let add = BitVec::from_bytes(b);
        if self.0.len() != add.len() {
            return false;
        }
        self.0.or(&add);
        true
    }

    pub fn finish(self) -> BitVec {
        self.0
    }
}
