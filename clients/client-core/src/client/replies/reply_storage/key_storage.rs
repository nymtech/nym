// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nymsphinx::anonymous_replies::SurbEncryptionKey;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
// TODO: we might have to also put the tag here
pub(crate) struct SentReplyKeys(Arc<HashSet<SurbEncryptionKey>>);

impl SentReplyKeys {
    //
}
