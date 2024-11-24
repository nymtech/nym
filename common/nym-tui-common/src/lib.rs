// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::panic)]

use crate::tui::manager::TuiManager;
use color_eyre::eyre;

pub mod error;
pub mod tui;

pub use crate::tui::config::TuiConfig;
pub use tui::action::{Action, AppAction};
pub use tui::dispatcher::store::State;
pub use tui::dispatcher::{ActionDispatcher, ActionSender};
pub use tui::initialize_panic_handler;
pub use tui::ui::components::Component;

// components:
pub use tui::ui::components::common::DebugHistory;
#[cfg(feature = "logger")]
pub use tui::ui::components::common::{Logger, LoggerProps};

pub async fn run_tui<C, D>(
    config: TuiConfig,
    action_dispatcher: D,
    initial_state: D::Store,
) -> eyre::Result<()>
where
    C: Component + Send + Sync + 'static,
    C::State: Send + Sync + 'static,
    C::Actions: Send + Sync + Clone + 'static,
    D: ActionDispatcher<Store = C::State, Actions = C::Actions> + Send + Sync + 'static,
{
    initialize_panic_handler()?;

    TuiManager::<C>::build_new(config, action_dispatcher, initial_state)?
        .wait_for_exit_or_signal()
        .await;

    Ok(())
}
