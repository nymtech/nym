// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tui::action::Action;
use crate::tui::config::TuiConfig;
use crate::tui::dispatcher::{ActionDispatcher, ActionSender, DispatcherLoop};
use crate::tui::ui::UiEventLoop;
use crate::Component;
use color_eyre::eyre;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{error, info};

pub struct TuiManager<C: Component> {
    shutdown_grace: Duration,
    cancel_grace: Duration,
    abort_grace: Duration,

    task_tracker: TaskTracker,
    cancellation_token: CancellationToken,

    dispatcher_handle: JoinHandle<eyre::Result<()>>,
    ui_event_loop_handle: JoinHandle<eyre::Result<()>>,

    action_sender: ActionSender<C::Actions>,
}

impl<C> TuiManager<C>
where
    C: Component + Send + Sync + 'static,
    C::State: Send + Sync + 'static,
    C::Actions: Send + Sync + Clone + 'static,
{
    pub(crate) fn build_new<D>(
        config: TuiConfig,
        action_dispatcher: D,
        initial_state: D::Store,
    ) -> eyre::Result<TuiManager<C>>
    where
        D: ActionDispatcher<Store = C::State, Actions = C::Actions> + Send + Sync + 'static,
    {
        let task_tracker = TaskTracker::new();
        let cancellation_token = CancellationToken::new();

        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let (state_tx, state_rx) = mpsc::unbounded_channel();

        let mut dispatcher_loop = DispatcherLoop::new(
            action_dispatcher,
            initial_state,
            action_tx.clone(),
            action_rx,
            state_tx,
            cancellation_token.clone(),
        );
        let mut ui_event_loop = UiEventLoop::<C>::new(
            config.tui.debug.tick_rate,
            cancellation_token.clone(),
            state_rx,
            action_tx.clone(),
        )?;

        let dispatcher_handle = task_tracker.spawn(async move { dispatcher_loop.run().await });
        let ui_event_loop_handle = task_tracker.spawn(async move { ui_event_loop.run().await });

        task_tracker.close();

        Ok(TuiManager {
            shutdown_grace: config.debug.shutdown_grace,
            cancel_grace: config.debug.cancel_grace,
            abort_grace: config.debug.abort_grace,
            task_tracker,
            cancellation_token,
            dispatcher_handle,
            ui_event_loop_handle,
            action_sender: action_tx.into(),
        })
    }

    async fn graceful_shutdown(&self) {
        // 1. try to send quit action to handle it the most gracefully
        self.action_sender.send(Action::Quit);
        if timeout(self.shutdown_grace, self.task_tracker.wait())
            .await
            .is_ok()
        {
            return;
        }
        error!("timed out while waiting for graceful shutdown");

        // 2. if that doesn't work, issue cancellation token
        self.cancellation_token.cancel();
        if timeout(self.cancel_grace, self.task_tracker.wait())
            .await
            .is_ok()
        {
            return;
        }
        error!("timed out while attempting to resolve cancellation token shutdown");

        // 3. finally go with nuclear option and just abort the tasks
        self.dispatcher_handle.abort();
        self.ui_event_loop_handle.abort();

        if timeout(self.abort_grace, self.task_tracker.wait())
            .await
            .is_ok()
        {
            return;
        }

        error!("somehow we still failed to shutdown our tasks! we might end up in a dirty state... oh well")
    }

    pub(crate) async fn wait_for_exit_or_signal(&self) {
        tokio::select! {
            _ = self.task_tracker.wait() => {
                // user decided to quit with 'normal' action
            }
            _ = wait_for_signal() => {
                self.graceful_shutdown().await
            }
        }
    }
}

#[cfg(unix)]
#[allow(clippy::expect_used)]
pub async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to setup SIGTERM channel");
    let mut sigquit = signal(SignalKind::quit()).expect("failed to setup SIGQUIT channel");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received SIGINT");
        },
        _ = sigterm.recv() => {
            info!("Received SIGTERM");
        }
        _ = sigquit.recv() => {
            info!("Received SIGQUIT");
        }
    }
}

#[cfg(not(unix))]
pub async fn wait_for_signal() {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received SIGINT");
        },
    }
}
