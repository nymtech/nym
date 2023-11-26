// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// based on https://github.com/ratatui-org/ratatui-async-template template

use crate::action::Action;
use crate::app::App;
use crate::cli::Args;

use crate::utils::initialize_panic_handler;
use clap::Parser;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::error;

pub mod action;
pub mod app;
pub mod cli;
pub mod components;
pub mod keybindings;
mod nyxd;
pub mod tui;
pub mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_panic_handler()?;

    let args = Args::parse();
    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    let (app, action_rx) = App::new(args).await?;
    // app.run().await?;
    run_app(app, action_rx).await
}

async fn run_app(mut app: App, mut action_rx: UnboundedReceiver<Action>) -> anyhow::Result<()> {
    let mut tui = tui::Tui::new()?;
    tui.enter()?;

    loop {
        // convert tui event to an application action
        if let Some(event) = tui.next().await {
            match event {
                tui::Event::Tick => app.tick(),
                tui::Event::Render => {
                    tui.draw(|f| {
                        if let Err(err) = app.draw(f) {
                            error!("failed to draw: {err:?}")
                        }
                    })?;
                }
                tui::Event::Resize(x, y) => {
                    tui.resize(Rect::new(0, 0, x, y))?;
                    tui.draw(|_f| {})?;
                }
                tui::Event::Key(key) => app.key_event(key)?,
                _ => {}
            }
        }

        // consume all emitted actions
        while let Ok(action) = action_rx.try_recv() {
            app.update(action)?
        }

        if app.should_quit {
            break;
        }
    }

    tui.exit()?;
    Ok(())
}
