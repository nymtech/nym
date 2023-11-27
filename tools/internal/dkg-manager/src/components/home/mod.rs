// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::Frame;
use crate::action::{Action, ActionSender};
use crate::action::{ContractsInfo, HomeAction};
use crate::components::home::dealers_info::draw_current_dealers_info;
use crate::components::home::dkg_info::draw_dkg_info;
use crate::components::home::dkg_timeconfig::draw_dkg_timeconfig;
use crate::components::home::group_info::draw_group_info;
use crate::nyxd::NyxdClient;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::{collections::HashMap, time::Duration};
use throbber_widgets_tui::ThrobberState;
use tokio::time::Instant;
use tracing::error;
use tui_input::{backend::crossterm::EventHandler, Input};
use url::Url;

pub(crate) mod dealers_info;
pub(crate) mod dkg_info;
pub(crate) mod dkg_timeconfig;
pub(crate) mod group_info;
pub(crate) mod utils;

pub const REFRESH_RATE: Duration = Duration::from_secs(60);

#[derive(Default, Clone, PartialEq, Eq)]
pub enum InputState {
    #[default]
    Normal,
    AddCW4Member {
        address: String,
    },
    RemoveCW4Member,
    AdvanceEpochState,
    SurpassedThreshold,

    Processing,
}

impl InputState {
    // will consume provided characters
    pub fn expects_text_input(&self) -> bool {
        match self {
            InputState::Normal => false,
            InputState::AddCW4Member { .. } => true,
            InputState::RemoveCW4Member => true,
            InputState::Processing => false,
            InputState::AdvanceEpochState => false,
            InputState::SurpassedThreshold => false,
        }
    }

    pub fn expects_user_input(&self) -> bool {
        !matches!(self, InputState::Normal | InputState::Processing)
    }

    pub fn next(&self) -> Self {
        match self {
            InputState::Normal => InputState::Normal,
            InputState::Processing => InputState::Processing,
            InputState::AddCW4Member { .. } => InputState::RemoveCW4Member,
            InputState::RemoveCW4Member => InputState::AdvanceEpochState,
            InputState::AdvanceEpochState => InputState::SurpassedThreshold,
            InputState::SurpassedThreshold => InputState::AddCW4Member {
                address: "".to_string(),
            },
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            InputState::Normal => InputState::Normal,
            InputState::Processing => InputState::Processing,
            InputState::AddCW4Member { .. } => InputState::SurpassedThreshold,
            InputState::RemoveCW4Member => InputState::AddCW4Member {
                address: "".to_string(),
            },
            InputState::AdvanceEpochState => InputState::RemoveCW4Member,
            InputState::SurpassedThreshold => InputState::AdvanceEpochState,
        }
    }
}

pub struct Home {
    pub show_help: bool,
    pub last_contract_error_message: String,
    pub refreshing: bool,
    throbber_state: Option<throbber_widgets_tui::ThrobberState>,

    pub nyxd_client: NyxdClient,
    pub manager_address: String,

    pub contracts: ContractsInfo,
    pub upstream: String,

    pub mode: InputState,
    pub input: Input,
    pub action_tx: ActionSender,
    pub keymap: HashMap<KeyEvent, Action>,
    pub last_contract_update: Instant,
}

impl Home {
    pub async fn new(
        nyxd_client: NyxdClient,
        upstream: Url,
        action_tx: ActionSender,
    ) -> anyhow::Result<Self> {
        let manager_address = nyxd_client.address().await.to_string();

        let contracts = nyxd_client.get_contract_update().await?;

        Ok(Home {
            show_help: false,

            nyxd_client,
            mode: Default::default(),
            input: Default::default(),
            action_tx,
            keymap: Default::default(),
            contracts,
            last_contract_update: Instant::now(),
            manager_address,
            last_contract_error_message: "".to_string(),
            refreshing: false,
            upstream: upstream.to_string(),
            throbber_state: None,
        })
    }

    pub fn keymap(mut self, keymap: HashMap<KeyEvent, Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn tick(&mut self) {
        // let state_end = OffsetDateTime::from_unix_timestamp(
        //     self.dkg_info.dkg_epoch.finish_timestamp.seconds() as i64,
        // )
        // .unwrap();
        // let until_epoch_state_end = state_end - OffsetDateTime::now_utc();
        // let epoch_should_move = until_epoch_state_end.as_seconds_f32() < -5.;

        // let should_refresh = !self.mode.expects_text_input()
        //     && (self.last_contract_update.elapsed() >= REFRESH_RATE || epoch_should_move);

        let should_refresh =
            !self.mode.expects_text_input() && self.last_contract_update.elapsed() >= REFRESH_RATE;

        if should_refresh && !self.refreshing {
            self.last_contract_update = Instant::now();
            self.refreshing = true;
            self.schedule_contract_refresh()
        }

        if let Some(throbber) = &mut self.throbber_state {
            throbber.calc_next()
        }
    }

    pub fn render_tick(&mut self) {}

    pub fn handle_input(&mut self, s: String) {
        match &self.mode {
            InputState::Normal | InputState::Processing => {
                panic!("received input whilst it shouldn't have been possible!")
            }
            InputState::AddCW4Member { address } => {
                if address.is_empty() {
                    self.mode = InputState::AddCW4Member { address: s };
                    self.input.reset();
                } else {
                    let address_owned = address.clone();
                    self.mode = InputState::Processing;
                    self.input.reset();
                    self.schedule_add_cw4_member(address_owned, s)
                }
            }
            InputState::RemoveCW4Member => {
                self.mode = InputState::Processing;
                self.input.reset();
                self.schedule_remove_cw4_member(s)
            }
            InputState::AdvanceEpochState => {
                self.mode = InputState::Processing;
                self.input.reset();
                self.schedule_call_advance_epoch_state();
            }
            InputState::SurpassedThreshold => {
                self.mode = InputState::Processing;
                self.input.reset();
                self.schedule_call_surpassed_threshold();
            }
        }
    }

    pub fn schedule_call_advance_epoch_state(&self) {
        let tx = self.action_tx.clone();
        let client = self.nyxd_client.clone();

        tokio::spawn(async move {
            match client.try_advance_epoch_state().await {
                Ok(_) => tx.unchecked_send_home_action(HomeAction::ScheduleContractRefresh),
                Err(err) => tx.unchecked_send_home_action(HomeAction::SetLastContractError(
                    format!("failed to advance epoch state: {err}"),
                )),
            }

            tx.unchecked_send_home_action(HomeAction::ExitProcessing)
        });
    }

    pub fn schedule_call_surpassed_threshold(&self) {
        let tx = self.action_tx.clone();
        let client = self.nyxd_client.clone();

        tokio::spawn(async move {
            match client.try_surpass_threshold().await {
                Ok(_) => {
                    tx.unchecked_send_home_action(HomeAction::ScheduleContractRefresh);
                }
                Err(err) => tx.unchecked_send_home_action(HomeAction::SetLastContractError(
                    format!("failed to surpass threshold: {err}"),
                )),
            }

            tx.unchecked_send_home_action(HomeAction::ExitProcessing);
        });
    }

    pub fn schedule_add_cw4_member(&self, member_address: String, member_weight_raw: String) {
        let tx = self.action_tx.clone();
        let client = self.nyxd_client.clone();

        tokio::spawn(async move {
            if let Ok(weight) = member_weight_raw.parse() {
                match client.add_group_member(member_address, weight).await {
                    Ok(_) => {
                        tx.unchecked_send_home_action(HomeAction::ScheduleContractRefresh);
                    }
                    Err(err) => tx.unchecked_send_home_action(HomeAction::SetLastContractError(
                        format!("failed to add group member: {err}"),
                    )),
                }
            } else {
                error!("could not parse '{member_weight_raw}' into a valid weight")
            }

            tx.unchecked_send_home_action(HomeAction::ExitProcessing);
        });
    }

    pub fn schedule_remove_cw4_member(&self, member_address: String) {
        let tx = self.action_tx.clone();
        let client = self.nyxd_client.clone();

        tokio::spawn(async move {
            match client.remove_group_member(member_address).await {
                Ok(_) => tx.unchecked_send_home_action(HomeAction::ScheduleContractRefresh),
                Err(err) => tx.unchecked_send_home_action(HomeAction::SetLastContractError(
                    format!("failed to remove group member: {err}"),
                )),
            }

            tx.unchecked_send_home_action(HomeAction::ExitProcessing)
        });
    }

    pub fn schedule_contract_refresh(&self) {
        let tx = self.action_tx.clone();
        let client = self.nyxd_client.clone();
        tokio::spawn(async move {
            tx.unchecked_send_home_action(HomeAction::EnterProcessing);
            match client.get_contract_update().await {
                Ok(info) => {
                    tx.unchecked_send_home_action(HomeAction::RefreshDkgContract(Box::new(info)))
                }
                Err(err) => {
                    error!("failed to get dkg updates: {err}")
                }
            }

            tx.unchecked_send_home_action(HomeAction::ExitProcessing);
            tx.unchecked_send_home_action(HomeAction::FinishContractUpdate)
        });
    }

    pub fn refresh_dkg_contract_info(&mut self, update_info: ContractsInfo) {
        self.contracts = update_info;
    }

    // fn contracts_info_lines(&self) -> Vec<Line> {
    //     vec![]
    //     //
    //     // if !self.last_contract_error_message.is_empty() {
    //     //     lines.push("".into());
    //     //     lines.push("".into());
    //     //     lines.push(Line::from(vec![
    //     //         Span::styled(
    //     //             "CONTRACT EXECUTION FAILURE: ",
    //     //             Style::default().red().bold(),
    //     //         ),
    //     //         Span::styled(&self.last_contract_error_message, Style::default().white()),
    //     //     ]))
    //     // }
    //     //
    //     // lines
    // }

    fn draw_main_widget(&mut self, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()> {
        let info_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(33), Constraint::Percentage(33)])
            .split(rect);

        let contract_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(info_chunks[0]);

        draw_group_info(
            &self.contracts.group,
            f,
            contract_chunks[0],
            self.matching_admin(),
        )?;
        draw_dkg_info(&self.contracts.dkg, f, contract_chunks[1])?;
        draw_dkg_timeconfig(
            self.contracts.dkg.epoch.time_configuration,
            f,
            contract_chunks[2],
        )?;

        draw_current_dealers_info(&self.contracts.dkg, f, info_chunks[1])?;

        // f.render_widget(self.main_widget(), rects[1]);

        Ok(())
    }

    fn matching_admin(&self) -> bool {
        if let Some(group_admin) = &self.contracts.group.admin.admin {
            return group_admin == &self.manager_address;
        }
        false
    }

    fn admin_span<S: Into<String>>(&self, text: S) -> Span {
        let style = if self.matching_admin() {
            Style::default().light_green().bold()
        } else {
            Style::default().light_red().bold()
        };
        Span::styled(text.into(), style)
    }

    fn title_widget(&self) -> Block {
        Block::default()
            .title(Line::from(vec![
                "Nym DKG Contract manager (managed via ".into(),
                self.admin_span(&self.manager_address),
                ") @ ".into(),
                Span::styled(&self.upstream, Style::new().light_blue()),
            ]))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(match self.mode {
                InputState::Processing => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            })
            .border_type(BorderType::Rounded)
    }

    fn input_widget(&self, scroll: usize) -> Paragraph {
        let mode_name = match &self.mode {
            InputState::Normal | InputState::Processing => Span::raw("Enter Input Mode "),
            InputState::AddCW4Member { address } => {
                if address.is_empty() {
                    Span::raw("Enter address of CW4 member to add to the group ")
                } else {
                    Span::raw(format!("Enter voting weight of new member '{address}' "))
                }
            }
            InputState::RemoveCW4Member => Span::raw("Enter address of CW4 member to remove it "),
            InputState::AdvanceEpochState => {
                Span::raw("press <ENTER> to attempt to advance the epoch state ")
            }
            InputState::SurpassedThreshold => {
                Span::raw("press <ENTER> to attempt to surpass the threshold ")
            }
        };

        Paragraph::new(self.input.value())
            .style(if self.mode.expects_user_input() {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .scroll((0, scroll as u16))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        mode_name,
                        Span::styled("(Press ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "/",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to start, ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "ESC",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to finish)", Style::default().fg(Color::DarkGray)),
                    ])),
            )
    }

    fn help_table(&self) -> Table {
        let rows = vec![
            Row::new(vec!["/", "Enter Input"]),
            Row::new(vec!["ESC", "Exit Input"]),
            Row::new(vec!["Enter", "Submit Input"]),
            Row::new(vec!["<Ctrl-c>", "Quit"]),
            Row::new(vec!["<Ctrl-d>", "Quit"]),
            Row::new(vec!["<Ctrl-h>", "Open Help"]),
            Row::new(vec!["<Ctrl-r>", "Force refresh contract state"]),
        ];
        Table::new(rows)
            .header(
                Row::new(vec!["Key", "Action"])
                    .bottom_margin(1)
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .widths(&[Constraint::Percentage(10), Constraint::Percentage(90)])
            .column_spacing(1)
    }

    pub fn handle_key_events(&mut self, key: KeyEvent) -> anyhow::Result<Option<Action>> {
        let action = match self.mode {
            InputState::Normal | InputState::Processing => None,
            _ => match key.code {
                KeyCode::Esc => Some(Action::HomeAction(HomeAction::EnterNormal)),
                KeyCode::Enter => Some(Action::HomeAction(HomeAction::ProcessInput(
                    self.input.value().to_string(),
                ))),
                KeyCode::Left => Some(Action::HomeAction(HomeAction::PreviousInputMode)),
                KeyCode::Right => Some(Action::HomeAction(HomeAction::NextInputMode)),
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    None
                }
            },
        };
        Ok(action)
    }

    pub fn update(&mut self, action: HomeAction) -> anyhow::Result<Option<Action>> {
        match action {
            HomeAction::ToggleShowHelp => self.show_help = !self.show_help,
            HomeAction::ScheduleContractRefresh => self.schedule_contract_refresh(),
            HomeAction::RefreshDkgContract(update_info) => {
                self.refresh_dkg_contract_info(*update_info)
            }
            HomeAction::ProcessInput(s) => self.handle_input(s),
            HomeAction::SetLastContractError(err) => self.last_contract_error_message = err,
            HomeAction::EnterNormal => {
                self.mode = InputState::Normal;
                self.last_contract_error_message = "".to_string();
            }
            HomeAction::StartInput => {
                // make sure we're not already in the input mode
                if !self.mode.expects_user_input() {
                    self.mode = InputState::AddCW4Member {
                        address: "".to_string(),
                    }
                }
            }
            HomeAction::PreviousInputMode => {
                self.mode = self.mode.previous();
                self.last_contract_error_message = "".to_string();
            }
            HomeAction::NextInputMode => {
                self.mode = self.mode.next();
                self.last_contract_error_message = "".to_string();
            }
            HomeAction::EnterProcessing => {
                self.mode = InputState::Processing;
                self.throbber_state = Some(ThrobberState::default());
                self.last_contract_error_message = "".to_string();
            }
            HomeAction::ExitProcessing => {
                self.mode = InputState::Normal;
                self.throbber_state = None;
            }
            HomeAction::FinishContractUpdate => {
                self.refreshing = false;
            }
        }
        Ok(None)
    }

    pub fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()> {
        let block = self.title_widget();
        let inner_area = block.inner(rect);
        f.render_widget(block, rect);

        let rects = Layout::default()
            .constraints(
                [
                    Constraint::Min(8),
                    Constraint::Percentage(100),
                    Constraint::Min(3),
                ]
                .as_ref(),
            )
            .split(inner_area);

        let info_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(rects[0]);

        self.contracts.bandwidth.base.draw(f, info_chunks[0])?;
        self.contracts.dkg.base.draw(f, info_chunks[1])?;
        self.contracts.group.base.draw(f, info_chunks[2])?;
        self.contracts.multisig.base.draw(f, info_chunks[3])?;

        self.draw_main_widget(f, rects[1])?;

        // old input logic (I haven't touched it yet)

        let width = rects[2].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);

        let input = self.input_widget(scroll);
        f.render_widget(input, rects[2]);

        if self.mode.expects_text_input() {
            f.set_cursor(
                (rects[2].x + 1 + self.input.cursor() as u16).min(rects[2].x + rects[2].width - 2),
                rects[2].y + 1,
            )
        }

        if let Some(throbber_state) = &mut self.throbber_state {
            let full = throbber_widgets_tui::Throbber::default()
                .label("Processing...")
                .style(Style::default().yellow())
                .throbber_style(Style::default().red().bold())
                .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                .use_type(throbber_widgets_tui::WhichUse::Spin);

            f.render_stateful_widget(
                full,
                Rect {
                    x: rect.x + 2,
                    y: rect.height,
                    width: rect.width.saturating_sub(2),
                    height: 1,
                },
                throbber_state,
            )
        }

        if self.show_help {
            let rect = rect.inner(&Margin {
                horizontal: 4,
                vertical: 2,
            });
            f.render_widget(Clear, rect);
            let block = Block::default()
                .title(Line::from(vec![Span::styled(
                    "Key Bindings",
                    Style::default().add_modifier(Modifier::BOLD),
                )]))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            f.render_widget(block, rect);

            f.render_widget(
                self.help_table(),
                rect.inner(&Margin {
                    vertical: 4,
                    horizontal: 2,
                }),
            );
        };

        Ok(())
    }
}
