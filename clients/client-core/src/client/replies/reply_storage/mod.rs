// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::client::replies::reply_storage::combined::CombinedReplyStorage;
pub use crate::client::replies::reply_storage::key_storage::SentReplyKeys;
pub use crate::client::replies::reply_storage::surb_storage::ReceivedReplySurbsMap;
pub use crate::client::replies::reply_storage::tag_storage::UsedSenderTags;
pub use backend::ReplyStorageBackend;

mod backend;
mod combined;
mod key_storage;
mod surb_storage;
mod tag_storage;

// only really exists to get information about shutdown and save data to the backing storage
pub struct ReplyStorage<T = backend::Empty>
where
    T: ReplyStorageBackend,
{
    combined_storage: CombinedReplyStorage,
    backend: T,
}

impl<T> Drop for ReplyStorage<T>
where
    T: ReplyStorageBackend,
{
    fn drop(&mut self) {
        println!("REPLY STORAGE IS GETTING DROPPED - WE SHOULD FLUSH ALL OUR DATA TO THE BACKING STORAGE!!")
        // todo!("flush everything to backend storage")
    }
}

impl<T> ReplyStorage<T> where T: ReplyStorageBackend {}
