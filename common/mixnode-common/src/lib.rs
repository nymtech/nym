// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
pub mod packet_processor;
pub mod verloc;

pub fn cpu_cycles() -> Result<i64, Box<dyn std::error::Error>> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "cpucycles")] {
            Ok(cpu_cycles::cpucycles()?)
        } else {
            Err("`cpucycles` feature is not turned on!".into())
        }
    }
}

#[macro_export]
macro_rules! measure {
    ( $x:expr ) => {{
        let start_cycles = $crate::cpu_cycles();
        // if the block needs to return something, we can return it
        let r = $x;
        let end_cycles = $crate::cpu_cycles();
        let name = if let Some(meta) = tracing::Span::current().metadata() {
            meta.name()
        } else {
            "measure"
        };
        match (start_cycles, end_cycles) {
            (Ok(start), Ok(end)) => info!("{} cpucycles: {}", name, end - start),
            (Err(e), _) => error!("{e}"),
            (_, Err(e)) => error!("{e}"),
        }
        r
    }};
}
