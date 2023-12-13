// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymRewarderError;
use std::ops::Add;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const HOUR: Duration = Duration::from_secs(60 * 60);

pub struct Epoch {
    pub id: i64,

    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

impl Epoch {
    pub const LENGTH: Duration = HOUR;

    pub fn first() -> Result<Self, NymRewarderError> {
        let start = OffsetDateTime::now_utc()
            .add(HOUR)
            .replace_nanosecond(0)?
            .replace_microsecond(0)?
            .replace_second(0)?;

        Ok(Epoch {
            id: 0,
            start,
            end: start + Self::LENGTH,
        })
    }

    pub fn until_end(&self) -> Duration {
        let now = OffsetDateTime::now_utc();
        (self.end - now).try_into().unwrap_or_default()
    }

    pub fn next(&self) -> Self {
        Epoch {
            id: self.id + 1,
            start: self.end,
            end: self.end + Self::LENGTH,
        }
    }

    pub fn start_rfc3339(&self) -> String {
        // safety: unwrap here is fine as we're using a predefined formatter
        #[allow(clippy::unwrap_used)]
        self.start.format(&Rfc3339).unwrap()
    }

    pub fn end_rfc3339(&self) -> String {
        // safety: unwrap here is fine as we're using a predefined formatter
        #[allow(clippy::unwrap_used)]
        self.end.format(&Rfc3339).unwrap()
    }
}
