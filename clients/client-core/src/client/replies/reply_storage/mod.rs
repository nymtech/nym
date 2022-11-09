// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) use crate::client::replies::reply_storage::{
    key_storage::SentReplyKeys, surb_storage::ReceivedReplySurbsMap,
};
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::{ReplySurb, SurbEncryptionKey};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

// TEMP
pub use crate::client::replies::reply_storage::combined::CombinedReplyStorage;

mod backend;
mod combined;
mod key_storage;
mod surb_storage;

// only really exists to get information about shutdown and save data to the backing storage
pub struct ReplyStorage<T> {
    combined_storage: CombinedReplyStorage,
    backend: T,
}

impl<T> Drop for ReplyStorage<T> {
    fn drop(&mut self) {
        println!("REPLY STORAGE IS GETTING DROPPED - WE SHOULD FLUSH ALL OUR DATA TO THE BACKING STORAGE!!")
        // todo!("flush everything to backend storage")
    }
}

impl<T> ReplyStorage<T> {
    // pub(crate) fn new(
    //     sent_reply_keys: SentReplyKeys,
    //     received_reply_surbs: ReceivedReplySurbsMap,
    //     backend: T,
    // ) -> Self {
    //     Self {
    //         sent_reply_keys,
    //         received_reply_surbs,
    //         backend,
    //     }
    // }
    //
    // pub(crate) fn received_reply_surbs(&self) -> ReceivedReplySurbsMap {
    //     self.received_reply_surbs.clone()
    // }
}
