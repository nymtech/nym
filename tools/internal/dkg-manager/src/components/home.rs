// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{Component, Frame};
use crate::action::ContractsInfo;
use crate::nyxd::NyxdClient;
use crate::{action::Action, utils::key_event_to_string};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::{collections::HashMap, time::Duration};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;
use tracing::{debug, error, info};
use tui_input::{backend::crossterm::EventHandler, Input};

pub const REFRESH_RATE: Duration = Duration::from_secs(60);

#[derive(Default, Clone, PartialEq, Eq)]
pub enum InputState {
    #[default]
    Normal,
    AddCW4Member {
        address: String,
    },
    RemoveCW4Member,

    Processing,
}

impl InputState {
    pub fn expects_input(&self) -> bool {
        match self {
            InputState::Normal => false,
            InputState::AddCW4Member { .. } => true,
            InputState::RemoveCW4Member => true,
            InputState::Processing => false,
        }
    }
}

pub struct Home {
    pub show_help: bool,
    pub counter: usize,
    pub app_ticker: usize,
    pub render_ticker: usize,

    pub nyxd_client: NyxdClient,
    pub manager_address: String,

    pub dkg_contract_address: String,
    pub group_contract_address: String,
    pub dkg_info: ContractsInfo,

    pub mode: InputState,
    pub input: Input,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub keymap: HashMap<KeyEvent, Action>,
    pub last_events: Vec<KeyEvent>,
    pub last_contract_update: Instant,
}

impl Home {
    pub async fn new(nyxd_client: NyxdClient) -> anyhow::Result<Self> {
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
            action_tx: None,
            keymap: Default::default(),
            last_events: vec![],
            dkg_info: initial_info,
            last_contract_update: Instant::now(),
            manager_address,
        })
    }

    pub fn keymap(mut self, keymap: HashMap<KeyEvent, Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn tick(&mut self) {
        info!("Tick");
        if self.last_contract_update.elapsed() >= REFRESH_RATE && !self.mode.expects_input() {
            self.last_contract_update = Instant::now();
            self.schedule_contract_refresh()
        }
        self.app_ticker = self.app_ticker.saturating_add(1);
        self.last_events.drain(..);
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
        }
    }

    pub fn schedule_add_cw4_member(&self, member_address: String, member_weight_raw: String) {
        let tx = self.action_tx.clone().unwrap();
        let client = self.nyxd_client.clone();

        tokio::spawn(async move {
            if let Ok(weight) = member_weight_raw.parse() {
                match client.add_group_member(member_address, weight).await {
                    Ok(_) => {
                        tx.send(Action::ScheduleContractRefresh).unwrap();
                    }
                    Err(err) => {
                        error!("failed to get add group member: {err}")
                    }
                }
            } else {
                error!("could not parse '{member_weight_raw}' into a valid weight")
            }

            tx.send(Action::ExitProcessing).unwrap();
        });
    }

    pub fn schedule_remove_cw4_member(&self, member_address: String) {
        let tx = self.action_tx.clone().unwrap();
        let client = self.nyxd_client.clone();

        tokio::spawn(async move {
            match client.remove_group_member(member_address).await {
                Ok(_) => {
                    tx.send(Action::ScheduleContractRefresh).unwrap();
                }
                Err(err) => {
                    error!("failed to get add group member: {err}")
                }
            }

            tx.send(Action::ExitProcessing).unwrap();
        });
    }

    pub fn schedule_contract_refresh(&self) {
        let tx = self.action_tx.clone().unwrap();
        let client = self.nyxd_client.clone();
        tokio::spawn(async move {
            tx.send(Action::EnterProcessing).unwrap();
            match client.get_dkg_update().await {
                Ok(info) => {
                    tx.send(Action::RefreshDkgContract(info)).unwrap();
                }
                Err(err) => {
                    error!("failed to get dkg updates: {err}")
                }
            }

            tx.send(Action::ExitProcessing).unwrap();
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

        let group_admin = match &info.group_admin.admin {
            None => Span::styled("<no admin>", Style::default().light_red().bold()),
            Some(admin) => Span::styled(admin.clone(), Style::default().light_green().bold()),
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
            ]),
            "".into(),
            // DKG config
            Span::styled("Time Configuration", Style::default().bold()).into(),
            Line::from(vec![
                "Public Key Submission: ".into(),
                Span::styled(
                    format!("{}secs", tc.public_key_submission_time_secs),
                    Style::default().yellow(),
                ),
            ]),
            Line::from(vec![
                "Dealing Exchange: ".into(),
                Span::styled(
                    format!("{}secs", tc.dealing_exchange_time_secs),
                    Style::default().yellow(),
                ),
            ]),
            Line::from(vec![
                "Verification Key Submission: ".into(),
                Span::styled(
                    format!("{}secs", tc.verification_key_submission_time_secs),
                    Style::default().yellow(),
                ),
            ]),
            Line::from(vec![
                "Verification Key Validation: ".into(),
                Span::styled(
                    format!("{}secs", tc.verification_key_validation_time_secs),
                    Style::default().yellow(),
                ),
            ]),
            Line::from(vec![
                "Verification Key Finalization: ".into(),
                Span::styled(
                    format!("{}secs", tc.verification_key_finalization_time_secs),
                    Style::default().yellow(),
                ),
            ]),
            Line::from(vec![
                "In Progress: ".into(),
                Span::styled(
                    format!("{}secs", tc.in_progress_time_secs),
                    Style::default().yellow(),
                ),
            ]),
            "".into(),
        ];

        // DKG threshold
        if let Some(threshold) = self.dkg_info.threshold {
            lines.push(Line::from(vec![
                "Threshold: ".into(),
                Span::styled(threshold.to_string(), Style::default().dim()),
            ]));
            lines.push("".into())
        }

        if !info.group_members.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("CW4 Group Members", Style::default().bold()),
                " (total weight: ".into(),
                Span::styled(
                    info.total_weight.weight.to_string(),
                    Style::default().yellow().bold(),
                ),
                ")".into(),
            ]));
            for member in &info.group_members {
                lines.push(Line::from(vec![
                    Span::styled(&member.addr, Style::default().bold()),
                    " (weight: ".into(),
                    Span::styled(member.weight.to_string(), Style::default().yellow()),
                    ")".into(),
                ]))
            }
        } else {
            lines.push(Span::styled("NO CW4 GROUP MEMBERS", Style::default().red().bold()).into())
        }

        lines.push("".into());

        // DKG dealers
        if !info.dealers.is_empty() {
            lines.push(Span::styled("Dkg Dealers", Style::default().bold()).into());
            for dealer in &info.dealers {
                let key_start = if dealer.bte_public_key_with_proof.len() < 16 {
                    dealer.bte_public_key_with_proof.clone()
                } else {
                    format!("{}...", &dealer.bte_public_key_with_proof[..16])
                };

                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", dealer.address), Style::default().bold()),
                    "\tindex: ".into(),
                    Span::styled(
                        format!("{}", dealer.assigned_index),
                        Style::default().bold().yellow(),
                    ),
                    "\t announce address: ".into(),
                    Span::styled(
                        dealer.announce_address.to_string(),
                        Style::default().bold().yellow(),
                    ),
                    "\t BTE key: ".into(),
                    Span::styled(key_start, Style::default().bold().yellow()),
                ]))
            }
        } else {
            lines.push(Span::styled("NO DKG DEALERS", Style::default().red().bold()).into())
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

    fn title_widget(&self) -> Block {
        Block::default()
            // .title("Nym DKG Contract manager")
            .title(Line::from(vec![
                "Nym DKG Contract manager (managed via ".into(),
                Span::styled(&self.manager_address, Style::default().light_green().bold()),
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
        };

        Paragraph::new(self.input.value())
            .style(match self.mode {
                InputState::AddCW4Member { .. } | InputState::RemoveCW4Member => {
                    Style::default().fg(Color::Yellow)
                }
                _ => Style::default(),
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

    fn history_widget(&self) -> Block {
        Block::default()
            .title(
                ratatui::widgets::block::Title::from(format!(
                    "{:?}",
                    &self
                        .last_events
                        .iter()
                        .map(key_event_to_string)
                        .collect::<Vec<_>>()
                ))
                .alignment(Alignment::Right),
            )
            .title_style(Style::default().add_modifier(Modifier::BOLD))
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
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> anyhow::Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> anyhow::Result<Option<Action>> {
        self.last_events.push(key);
        let action = match self.mode {
            InputState::Normal | InputState::Processing => return Ok(None),
            InputState::AddCW4Member { .. } | InputState::RemoveCW4Member => match key.code {
                KeyCode::Esc => Action::EnterNormal,
                KeyCode::Enter => Action::ProcessInput(self.input.value().to_string()),
                KeyCode::Left => Action::PreviousInputMode,
                KeyCode::Right => Action::NextInputMode,
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    Action::Update
                }
            },
        };
        Ok(Some(action))
    }

    fn update(&mut self, action: Action) -> anyhow::Result<Option<Action>> {
        match action {
            Action::Tick => self.tick(),
            Action::Render => self.render_tick(),
            Action::ToggleShowHelp => self.show_help = !self.show_help,
            Action::ScheduleContractRefresh => self.schedule_contract_refresh(),
            Action::RefreshDkgContract(update_info) => self.refresh_dkg_contract_info(update_info),
            Action::ProcessInput(s) => self.handle_input(s),
            Action::EnterNormal => {
                self.mode = InputState::Normal;
            }
            Action::StartInput => {
                // make sure we're not already in the input mode
                if !self.mode.expects_input() {
                    self.mode = InputState::AddCW4Member {
                        address: "".to_string(),
                    }
                }
            }
            Action::PreviousInputMode => {
                if matches!(self.mode, InputState::RemoveCW4Member) {
                    self.mode = InputState::AddCW4Member {
                        address: "".to_string(),
                    }
                } else {
                    self.mode = InputState::RemoveCW4Member
                }
            }
            Action::NextInputMode => {
                if matches!(self.mode, InputState::RemoveCW4Member) {
                    self.mode = InputState::AddCW4Member {
                        address: "".to_string(),
                    }
                } else {
                    self.mode = InputState::RemoveCW4Member
                }
            }
            // Action::EnterCW4AddMember => {
            //     self.mode = InputState::AddCW4Member {
            //         address: "".to_string(),
            //     }
            // }
            // Action::EnterCW4AddMemberWeight { address } => {
            //     self.mode = InputState::AddCW4Member { address }
            // }
            // Action::EnterCW4RemoveMember => {
            //     self.mode = InputState::RemoveCW4Member;
            // }
            Action::EnterProcessing => {
                self.mode = InputState::Processing;
            }
            Action::ExitProcessing => {
                self.mode = InputState::Normal;
            }
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()> {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100), Constraint::Min(3)].as_ref())
            .split(rect);

        f.render_widget(self.main_widget(), rects[0]);

        let width = rects[1].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);

        let input = self.input_widget(scroll);
        f.render_widget(input, rects[1]);

        if self.mode.expects_input() {
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

        f.render_widget(
            self.history_widget(),
            Rect {
                x: rect.x + 1,
                y: rect.height.saturating_sub(1),
                width: rect.width.saturating_sub(2),
                height: 1,
            },
        );

        Ok(())
    }
}
