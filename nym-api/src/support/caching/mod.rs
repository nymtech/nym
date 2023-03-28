use serde::Serialize;
use std::ops::Deref;
use time::OffsetDateTime;

#[derive(Default, Serialize, Clone)]
pub struct Cache<T> {
    pub value: T,
    as_at: i64,
}

impl<T: Clone> Cache<T> {
    pub fn new(value: T) -> Self {
        Cache {
            value,
            as_at: current_unix_timestamp(),
        }
    }

    pub(crate) fn update(&mut self, value: T) {
        self.value = value;
        self.as_at = current_unix_timestamp()
    }

    pub fn timestamp(&self) -> i64 {
        self.as_at
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> Deref for Cache<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

fn current_unix_timestamp() -> i64 {
    let now = OffsetDateTime::now_utc();
    now.unix_timestamp()
}

// The cache can emit notifications to listeners about the current state
#[derive(Debug, PartialEq, Eq)]
pub enum CacheNotification {
    Start,
    Updated,
}
