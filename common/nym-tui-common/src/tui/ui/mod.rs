// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tui::dispatcher::{ActionSender, StateUpdateReceiver};
use crate::tui::handle::TuiHandle;
use crate::tui::ui::components::Component;
use color_eyre::eyre;
use color_eyre::eyre::eyre;
use crossterm::event::Event;
use humantime_serde::re::humantime;
use std::io;
use std::marker::PhantomData;
use std::time::Duration;
use tokio::time::{timeout, Instant};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{info, trace, warn};

pub mod components;

pub struct UiEventLoop<C: Component> {
    tick_rate: Duration,
    cancellation_token: CancellationToken,
    state_receiver: StateUpdateReceiver<C::State>,

    // only to be used to construct root 'App' instance
    action_sender: ActionSender<C::Actions>,
    tui_handle: TuiHandle,

    root_component: PhantomData<C>,
}

impl<C> UiEventLoop<C>
where
    C: Component,
{
    pub fn new(
        tick_rate: Duration,
        cancellation_token: CancellationToken,
        state_receiver: impl Into<StateUpdateReceiver<C::State>>,
        action_sender: impl Into<ActionSender<C::Actions>>,
    ) -> eyre::Result<Self> {
        Ok(UiEventLoop {
            tick_rate,
            cancellation_token,
            state_receiver: state_receiver.into(),
            action_sender: action_sender.into(),
            tui_handle: TuiHandle::new()?,
            root_component: PhantomData,
        })
    }

    async fn handle_ui_event(
        &mut self,
        event: Option<io::Result<Event>>,
        root_app: &mut C,
    ) -> eyre::Result<()> {
        let Some(event) = event else {
            warn!("the crossterm event channel has closed! we're probably already in shutdown!");
            // but if we're not, make sure to kick it off...
            self.cancellation_token.cancel();
            return Ok(());
        };

        match event? {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Key(key_event) => root_app.handle_key(key_event)?,
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }

        Ok(())
    }

    async fn handle_updated_state(&mut self, state: Option<C::State>, root_app: C) -> C
    where
        C: Component,
    {
        let Some(updated_state) = state else {
            warn!("the state update channel has closed! we're probably already in shutdown!");
            // but if we're not, make sure to kick it off...
            self.cancellation_token.cancel();
            return root_app;
        };

        root_app.update(&updated_state)
    }

    pub async fn run(&mut self) -> eyre::Result<()>
    where
        // this clone shouldn't really be needed...
        C::Actions: Clone,
    {
        info!("starting the ui loop");

        // wait for initial state...
        let initial_state = timeout(Duration::from_secs(1), self.state_receiver.next())
            .await?
            .ok_or_else(|| eyre!("did not receive initial state!"))?;

        let mut root_app = C::new(&initial_state, self.action_sender.clone());

        let mut tick_rate = self.tick_rate;
        let mut tick_interval = tokio::time::interval(tick_rate);
        self.tui_handle.enter()?;

        let mut draw = true;

        loop {
            if draw {
                let draw_start = Instant::now();

                trace!("redrawing the UI");
                self.tui_handle
                    .draw(|frame| root_app.view(frame, frame.area()))?;

                let taken = humantime::format_duration(draw_start.elapsed()).to_string();
                trace!(time_taken = taken, "UI drawing");
            }

            tokio::select! {
                biased;
                _ = self.cancellation_token.cancelled() => {
                    info!("received cancellation token");
                    break;
                }
                maybe_ui_event = self.tui_handle.next() => {
                    self.handle_ui_event(maybe_ui_event, &mut root_app).await?;
                    draw = true;
                }
                state_update = self.state_receiver.next() => {
                    root_app = self.handle_updated_state(state_update, root_app).await;
                    // the tick rate has changed
                    if self.tick_rate != tick_rate {
                        tick_rate = self.tick_rate;
                        tick_interval = tokio::time::interval(tick_rate);
                    }
                    draw = true;
                }
                _ = tick_interval.tick() => {
                    let tick_start = Instant::now();
                    draw = root_app.tick();

                    let taken = humantime::format_duration(tick_start.elapsed()).to_string();
                    trace!(time_taken = taken, will_redraw = draw, "app tick");
                },
            }
        }

        Ok(())
    }
}
