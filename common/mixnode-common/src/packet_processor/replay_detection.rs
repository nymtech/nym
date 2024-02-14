// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::packet_processor::error::MixProcessingError;
use fastbloom_rs::{BloomFilter, FilterBuilder, Membership};
use std::sync::{Arc, Mutex};

const BLOOM_FILTER_SIZE: u64 = 10_000_000;
const FP_RATE: f64 = 1e-4;

//alias for convenience
type ReplayTag = [u8];

#[derive(Clone, Debug)]
pub struct ReplayDetector(Arc<Mutex<ReplayDetectorInner>>);

impl ReplayDetector {
    pub fn new() -> Self {
        ReplayDetector(Arc::new(Mutex::new(ReplayDetectorInner::new())))
    }

    //check if secret has been seen already
    //if no, return Ok
    //if yes, add the secret to the list, then return an error
    pub fn handle_replay_tag(&self, replay_tag: &ReplayTag) -> Result<(), MixProcessingError> {
        match self.0.lock() {
            Ok(mut inner) => {
                if !inner.lookup_then_insert(replay_tag) {
                    Ok(())
                } else {
                    Err(MixProcessingError::ReplayedPacketDetected)
                }
            }
            Err(err) => {
                log::warn!("Failed to handle replay_tag : {err}");
                Ok(()) //what is the sensible thing to do, if the lock is poisoned? Reset the filter ?
            }
        }
    }
}

impl Default for ReplayDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct ReplayDetectorInner {
    filter: BloomFilter,
}

impl ReplayDetectorInner {
    pub fn new() -> Self {
        ReplayDetectorInner {
            filter: FilterBuilder::new(BLOOM_FILTER_SIZE, FP_RATE).build_bloom_filter(),
        }
    }

    pub fn lookup_then_insert(&mut self, replay_tag: &ReplayTag) -> bool {
        self.filter.contains_then_add(replay_tag)
    }
}

#[cfg(test)]
mod replay_detector_test {
    use super::*;

    #[test]
    fn handle_replay_tag_correctly_detects_replay() {
        let replay_detector = ReplayDetector::new();
        let replay_tag = b"Hello World!";
        assert!(replay_detector.handle_replay_tag(replay_tag).is_ok()); //first insert is fine
        assert!(replay_detector.handle_replay_tag(replay_tag).is_err()); //second is not
    }
}
