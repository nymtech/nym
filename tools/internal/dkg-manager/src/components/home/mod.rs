// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::Frame;
use crate::action::{Action, ActionSender};
use crate::action::{ContractsInfo, HomeAction};
use crate::components::home::utils::{
    cw4_members_header, format_cw4_member, format_dealer, format_dealing, format_time_configuration,
};
use crate::nyxd::NyxdClient;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::{collections::HashMap, time::Duration};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::time::Instant;
use tracing::{debug, error, info};
use tui_input::{backend::crossterm::EventHandler, Input};

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
    pub counter: usize,
    pub app_ticker: usize,
    pub render_ticker: usize,
    pub last_contract_error_message: String,

    pub nyxd_client: NyxdClient,
    pub manager_address: String,

    pub dkg_contract_address: String,
    pub group_contract_address: String,
    pub dkg_info: ContractsInfo,

    pub mode: InputState,
    pub input: Input,
    pub action_tx: ActionSender,
    pub keymap: HashMap<KeyEvent, Action>,
    pub last_contract_update: Instant,
}

impl Home {
    pub async fn new(nyxd_client: NyxdClient, action_tx: ActionSender) -> anyhow::Result<Self> {
        let dkg_contract_address = nyxd_client.dkg_contract().await?.to_string();

        let group_contract_address = nyxd_client.group_contract().await?.to_string();
        let manager_address = nyxd_client.address().await.to_string();

        let initial_info = nyxd_client.get_dkg_update().await?;

        Ok(Home {
            show_help: false,
            counter: 0,
            app_ticker: 0,
            render_ticker: 0,
            dkg_contract_address,
            group_contract_address,
            nyxd_client,
            mode: Default::default(),
            input: Default::default(),
            action_tx,
            keymap: Default::default(),
            dkg_info: initial_info,
            last_contract_update: Instant::now(),
            manager_address,
            last_contract_error_message: "".to_string(),
        })
    }

    pub fn keymap(mut self, keymap: HashMap<KeyEvent, Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn tick(&mut self) {
        info!("Tick");
        let state_end = OffsetDateTime::from_unix_timestamp(
            self.dkg_info.dkg_epoch.finish_timestamp.seconds() as i64,
        )
        .unwrap();
        let until_epoch_state_end = state_end - OffsetDateTime::now_utc();
        let epoch_should_move = until_epoch_state_end.as_seconds_f32() < -5.;

        let should_refresh = !self.mode.expects_text_input()
            && (self.last_contract_update.elapsed() >= REFRESH_RATE || epoch_should_move);

        if should_refresh {
            self.last_contract_update = Instant::now();
            self.schedule_contract_refresh()
        }
        self.app_ticker = self.app_ticker.saturating_add(1);
    }

    pub fn render_tick(&mut self) {
        debug!("Render Tick");
        self.render_ticker = self.render_ticker.saturating_add(1);
    }

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
            match client.get_dkg_update().await {
                Ok(info) => {
                    tx.unchecked_send_home_action(HomeAction::RefreshDkgContract(Box::new(info)))
                }
                Err(err) => {
                    error!("failed to get dkg updates: {err}")
                }
            }

            tx.unchecked_send_home_action(HomeAction::ExitProcessing)
        });
    }

    pub fn refresh_dkg_contract_info(&mut self, update_info: ContractsInfo) {
        self.dkg_info = update_info;
    }

    fn contracts_info_lines(&self) -> Vec<Line> {
        let info = &self.dkg_info;

        let tc = info.dkg_epoch.time_configuration;
        let end =
            OffsetDateTime::from_unix_timestamp(info.dkg_epoch.finish_timestamp.seconds() as i64)
                .unwrap();

        let remaining = end - OffsetDateTime::now_utc();
        let remaining_str = if remaining.is_negative() {
            format!("-{} secs", remaining.whole_seconds())
        } else {
            format!("{} secs", remaining.whole_seconds())
        };

        let group_admin = match &info.group_admin.admin {
            None => Span::styled("<no admin>", Style::default().light_red().bold()),
            Some(admin) => self.admin_span(admin),
        };

        let mut lines = vec![
            // contract addresses
            Line::from(vec![
                "Dkg Contract is at: ".into(),
                Span::styled(&self.dkg_contract_address, Style::default().yellow().bold()),
            ]),
            Line::from(vec![
                "Group Contract is at: ".into(),
                Span::styled(
                    &self.group_contract_address,
                    Style::default().yellow().bold(),
                ),
                " admin: ".into(),
                group_admin,
            ]),
            Line::from(vec![
                "Dkg Debug State: ".into(),
                Span::styled(format!("{:?}", info.dkg_state), Style::default().dim()),
            ]),
            "".into(),
            // DKG epoch state
            Line::from(vec![
                format!("DKG Epoch {} State: ", info.dkg_epoch.epoch_id).into(),
                Span::styled(info.dkg_epoch.state.to_string(), Style::default().bold()),
            ]),
            Line::from(vec![
                "End time: ".into(),
                Span::styled(
                    end.format(&Rfc3339).unwrap().to_string(),
                    Style::default().bold(),
                ),
                " (".into(),
                Span::styled(remaining_str, Style::default().light_green()),
                " remaining)".into(),
            ]),
            "".into(),
        ];

        // DKG config
        lines.append(&mut format_time_configuration(tc));
        lines.push("".into());

        // DKG threshold
        if let Some(threshold) = self.dkg_info.threshold {
            lines.push(Line::from(vec![
                "Threshold: ".into(),
                Span::styled(threshold.to_string(), Style::default().bold()),
            ]));
            lines.push("".into())
        }

        if !info.group_members.is_empty() {
            lines.push(cw4_members_header(&info.total_weight));
            for member in &info.group_members {
                lines.push(format_cw4_member(member))
            }
        } else {
            lines.push(Span::styled("NO CW4 GROUP MEMBERS", Style::default().red().bold()).into())
        }

        lines.push("".into());

        // DKG dealers
        if !info.dealers.is_empty() {
            lines.push(Span::styled("Dkg Dealers", Style::default().bold()).into());
            for dealer in &info.dealers {
                lines.push(format_dealer(dealer))
            }
        } else {
            lines.push(Span::styled("NO DKG DEALERS", Style::default().red().bold()).into())
        }

        lines.push("".into());

        if !info.past_dealers.is_empty() {
            lines.push(Span::styled("Past Dkg Dealers", Style::default().bold()).into());
            for dealer in &info.past_dealers {
                lines.push(format_dealer(dealer))
            }
            lines.push("".into());
        }

        if !info.epoch_dealings.is_empty() {
            lines.push(Span::styled("Epoch dealings", Style::default().bold()).into());

            for dealing in &info.epoch_dealings {
                lines.push(format_dealing(dealing))
            }
            lines.push("".into());
        }

        if !self.last_contract_error_message.is_empty() {
            lines.push("".into());
            lines.push("".into());
            lines.push(Line::from(vec![
                Span::styled(
                    "CONTRACT EXECUTION FAILURE: ",
                    Style::default().red().bold(),
                ),
                Span::styled(&self.last_contract_error_message, Style::default().white()),
            ]))
        }

        lines
    }

    fn main_widget(&self) -> Paragraph {
        let mut text: Vec<Line> = Vec::new();
        text.push("".into());
        text.push(format!("[debug] Render Ticker: {}", self.render_ticker).into());
        text.push(format!("[debug] App Ticker: {}", self.app_ticker).into());
        text.push("".into());
        text.append(&mut self.contracts_info_lines());

        Paragraph::new(text)
            .block(self.title_widget())
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
    }

    fn matching_admin(&self) -> bool {
        if let Some(group_admin) = &self.dkg_info.group_admin.admin {
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
                ")".into(),
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
                self.last_contract_error_message = "".to_string();
            }
            HomeAction::ExitProcessing => {
                self.mode = InputState::Normal;
            }
            _ => (),
        }
        Ok(None)
    }

    pub fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()> {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100), Constraint::Min(3)].as_ref())
            .split(rect);

        f.render_widget(self.main_widget(), rects[0]);

        let width = rects[1].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);

        let input = self.input_widget(scroll);
        f.render_widget(input, rects[1]);

        if self.mode.expects_text_input() {
            f.set_cursor(
                (rects[1].x + 1 + self.input.cursor() as u16).min(rects[1].x + rects[1].width - 2),
                rects[1].y + 1,
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
