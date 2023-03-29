// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod packet_processor;
pub mod verloc;

#[cfg(feature = "cpucycles")]
pub fn cpu_cycles() -> i64 {
    cpu_cycles::cpucycles().unwrap_or(-1)
}

#[cfg(not(feature = "cpucycles"))]
pub fn cpu_cycles() -> i64 {
    0
}

#[macro_export]
macro_rules! measure {
    ( $x:expr ) => {{
        let start_cycles = $crate::cpu_cycles();
        // if the block needs to return something, we can return it
        let r = $x;
        let end_cycles = $crate::cpu_cycles();
        info!("{:?}", tracing::Span::current());
        tracing::Span::current().record("cpucycles", end_cycles - start_cycles);
        r
    }};
}
