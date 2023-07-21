use std::env;

use anyhow::{Context, Result};
use sentry::ClientInitGuard;

use crate::constants::{SENTRY_DSN_JS, SENTRY_DSN_RUST};

pub fn init() -> Result<ClientInitGuard> {
    // if these env vars were set at compile time, use their value
    if let Some(v) = option_env!("SENTRY_DSN_RUST") {
        env::set_var(SENTRY_DSN_RUST, v);
    }
    if let Some(v) = option_env!("SENTRY_DSN_JS") {
        env::set_var(SENTRY_DSN_JS, v);
    }

    let dsn = env::var(SENTRY_DSN_RUST).context(format!("{} env var not set", SENTRY_DSN_RUST))?;
    println!("using DSN {dsn}");
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            sample_rate: 1.0, // TODO lower this in prod
            traces_sample_rate: 1.0,
            ..Default::default() // TODO add data scrubbing
                                 // see https://docs.sentry.io/platforms/rust/data-management/sensitive-data/
        },
    ));

    sentry::configure_scope(|scope| {
        scope.set_user(Some(sentry::User {
            id: Some("nym".into()),
            ..Default::default()
        }));
    });

    Ok(guard)
}
