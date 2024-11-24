// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tui::action::Action;
use crate::tui::dispatcher::store::State;
use crate::AppAction;
use async_trait::async_trait;
use color_eyre::eyre;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub mod store;

#[derive(Clone)]
pub struct ActionSender<T: AppAction>(UnboundedSender<Action<T>>);

impl<T> From<UnboundedSender<Action<T>>> for ActionSender<T>
where
    T: AppAction,
{
    fn from(value: UnboundedSender<Action<T>>) -> Self {
        ActionSender(value)
    }
}

impl<T> ActionSender<T>
where
    T: AppAction,
{
    pub fn send(&self, action: impl Into<Action<T>>) {
        if let Err(unsent) = self.0.send(action.into()) {
            error!("failed to send {:?} action to the dispatcher", unsent.0)
        }
    }
}

pub type ActionReceiver<T> = UnboundedReceiverStream<Action<T>>;

pub type StateUpdateSender<S> = UnboundedSender<S>;
pub type StateUpdateReceiver<S> = UnboundedReceiverStream<S>;

#[async_trait]
pub trait ActionDispatcher {
    type Store: State;
    type Actions: AppAction;

    async fn handle_app_action(
        &mut self,
        action: Self::Actions,
        store: &mut Self::Store,
    ) -> eyre::Result<()>;
}

pub struct DispatcherLoop<D: ActionDispatcher> {
    dispatcher: D,
    store: D::Store,

    // to be used with async actions
    #[allow(dead_code)]
    action_sender: ActionSender<D::Actions>,
    action_receiver: ActionReceiver<D::Actions>,
    state_update_sender: StateUpdateSender<D::Store>,
    cancellation_token: CancellationToken,
}

impl<D> DispatcherLoop<D>
where
    D: ActionDispatcher + Send + Sync + 'static,
    D::Store: Send + Sync + 'static,
{
    pub(crate) fn new(
        dispatcher: D,
        store: D::Store,
        action_sender: impl Into<ActionSender<D::Actions>>,
        action_receiver: impl Into<ActionReceiver<D::Actions>>,
        state_update_sender: StateUpdateSender<D::Store>,
        cancellation_token: CancellationToken,
    ) -> DispatcherLoop<D> {
        DispatcherLoop {
            dispatcher,
            store,
            action_sender: action_sender.into(),
            action_receiver: action_receiver.into(),
            state_update_sender,
            cancellation_token,
        }
    }

    async fn handle_action(&mut self, action: Option<Action<D::Actions>>) -> eyre::Result<()> {
        let Some(action) = action else {
            warn!("the dispatcher channel has closed! we're probably already in shutdown!");
            // but if we're not, make sure to kick it off...
            self.cancellation_token.cancel();
            return Ok(());
        };

        match action {
            Action::Quit => {
                debug!("attempting to handle the QUIT action");
                self.cancellation_token.cancel();
                // no need to send any state updates here
                return Ok(());
            }
            Action::AppDefined(action) => {
                debug!("attempting to handle the following action: {:?}", action);
                self.dispatcher
                    .handle_app_action(action, &mut self.store)
                    .await?;
            }
        }

        self.state_update_sender.send(self.store.clone())?;
        Ok(())
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        info!("starting the dispatcher loop");

        // issue initial state
        self.state_update_sender.send(self.store.clone())?;

        loop {
            tokio::select! {
                biased;
                _ = self.cancellation_token.cancelled() => {
                    info!("received cancellation token");
                    break;
                }
                maybe_action = self.action_receiver.next() => {
                    self.handle_action(maybe_action).await?
                }
            }
        }
        Ok(())
    }
}
