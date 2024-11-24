// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use color_eyre::eyre;
use crossterm::event::{KeyCode, KeyEvent};
use nym_tui_common::tui::config::keybindings::{KeyBinding, LoggerKeybindings};
use nym_tui_common::{
    run_tui, Action, ActionDispatcher, ActionSender, AppAction, Component, DebugHistory, Logger,
    LoggerProps, State,
};
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Constraint;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tracing::log::LevelFilter;
use tracing_subscriber::EnvFilter;

// useful pattern for tabs, etc:
/*
fn get_active_page_component_mut(&mut self) -> &mut dyn Component {
    match self.props.active_tab {
        ActiveTab::Tab1 => &mut self.tab1,
        ActiveTab::Tab2 => &mut self.tab2,
    }
}
 */

struct Props {
    custom_quit: KeyBinding,
}

impl From<&HelloStore> for Props {
    fn from(store: &HelloStore) -> Self {
        Props {
            custom_quit: store.config.custom_quit,
        }
    }
}

pub struct HelloRootApp {
    props: Props,

    action_sender: ActionSender<HelloActions>,
    logger: Logger<HelloStore, HelloActions>,
    debug_history: DebugHistory<HelloStore, HelloActions>,
}

impl Component for HelloRootApp {
    type State = HelloStore;
    type Actions = HelloActions;

    fn new(state: &HelloStore, action_sender: ActionSender<HelloActions>) -> Self
    where
        Self: Sized,
    {
        HelloRootApp {
            props: Props::from(state),
            action_sender: action_sender.clone(),
            logger: Logger::new(state, action_sender.clone()),
            debug_history: DebugHistory::new(state, action_sender),
        }
    }

    fn update(self, state: &Self::State) -> Self
    where
        Self: Sized,
    {
        HelloRootApp {
            logger: self.logger.update(state),
            debug_history: self.debug_history.update(state),
            ..self
        }
    }

    fn tick(&mut self) -> bool {
        let logger_tick = self.logger.tick();
        let debug_history_tick = self.debug_history.tick();

        logger_tick || debug_history_tick
    }

    fn handle_key(&mut self, key: KeyEvent) -> eyre::Result<()> {
        let maybe_binding = KeyBinding::from(key);
        if maybe_binding == self.props.custom_quit {
            self.action_sender.send(Action::Quit);
        }

        self.logger.handle_key(key)?;
        self.debug_history.handle_key(key)?;

        Ok(())
    }

    fn view(&mut self, frame: &mut Frame, rect: Rect) {
        let [logs, hello_rect] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(rect);

        self.logger.view(frame, logs);

        frame.render_widget(Paragraph::new("Hello world!").centered(), hello_rect);

        self.debug_history.view(frame, rect);
    }
}

#[derive(Debug, Clone)]
pub enum HelloActions {}

impl AppAction for HelloActions {}

#[derive(Clone)]
pub struct HelloConfig {
    pub custom_quit: KeyBinding,
    pub logger_keybindings: LoggerKeybindings,
}

impl Default for HelloConfig {
    fn default() -> Self {
        HelloConfig {
            custom_quit: KeyBinding::new(KeyCode::Char('x')),
            logger_keybindings: Default::default(),
        }
    }
}

#[derive(Clone, Default)]
pub struct HelloStore {
    config: HelloConfig,
}

impl<'a> From<&'a HelloStore> for LoggerProps {
    fn from(store: &'a HelloStore) -> LoggerProps {
        LoggerProps {
            keybindings: store.config.logger_keybindings,
        }
    }
}

impl State for HelloStore {}

#[derive(Default)]
pub struct HelloDispatcher {}

#[async_trait]
impl ActionDispatcher for HelloDispatcher {
    type Store = HelloStore;
    type Actions = HelloActions;

    async fn handle_app_action(
        &mut self,
        action: Self::Actions,
        store: &mut Self::Store,
    ) -> eyre::Result<()> {
        let _ = action;
        let _ = store;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    let filter: EnvFilter = "trace,mio=warn".parse()?;

    tracing_subscriber::registry()
        .with(tui_logger::tracing_subscriber_layer().with_filter(filter))
        .init();
    tui_logger::init_logger(LevelFilter::Trace)?;

    run_tui::<HelloRootApp, _>(Default::default(), HelloDispatcher {}, Default::default()).await
}
