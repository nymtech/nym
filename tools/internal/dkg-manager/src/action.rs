// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::DkgState;
use nym_coconut_dkg_common::dealer::{ContractDealing, DealerDetails};
use nym_coconut_dkg_common::types::Epoch;
use nym_validator_client::nyxd::{cw4, cw_controllers};
use serde::{Serialize, Serializer};

use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::UnboundedSender;
use tui_logger::TuiWidgetEvent;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractsInfo {
    // dkg details
    pub dkg_epoch: Epoch,
    pub threshold: Option<u64>,
    pub dealers: Vec<DealerDetails>,
    pub past_dealers: Vec<DealerDetails>,
    pub epoch_dealings: Vec<ContractDealing>,
    pub dkg_state: DkgState,

    // group details
    pub group_admin: cw_controllers::AdminResponse,
    pub group_members: Vec<cw4::Member>,
    pub total_weight: cw4::TotalWeightResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Action {
    Quit,
    Error(String),
    NextTab,
    PreviousTab,

    HomeAction(HomeAction),
    LoggerAction(LoggerAction),
}

impl From<HomeAction> for Action {
    fn from(value: HomeAction) -> Self {
        Action::HomeAction(value)
    }
}

impl From<LoggerAction> for Action {
    fn from(value: LoggerAction) -> Self {
        Action::LoggerAction(value)
    }
}

impl From<TuiWidgetEvent> for Action {
    fn from(value: TuiWidgetEvent) -> Self {
        LoggerAction::WidgetKeyEvent(value).into()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum HomeAction {
    ToggleShowHelp,
    StartInput,
    ScheduleContractRefresh,
    RefreshDkgContract(Box<ContractsInfo>),
    ProcessInput(String),
    SetLastContractError(String),
    EnterNormal,
    EnterCW4AddMember,
    // EnterCW4AddMemberWeight { address: String },
    // EnterCW4RemoveMember,
    NextInputMode,
    PreviousInputMode,

    EnterProcessing,
    ExitProcessing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoggerAction {
    WidgetKeyEvent(TuiWidgetEvent),
}

impl Serialize for LoggerAction {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        todo!()
    }
}

#[derive(Clone)]
pub struct ActionSender(pub UnboundedSender<Action>);

impl ActionSender {
    pub fn send(&self, action: Action) -> Result<(), SendError<Action>> {
        self.0.send(action)
    }

    pub fn send_home_action(&self, action: HomeAction) -> Result<(), SendError<Action>> {
        self.send(Action::HomeAction(action))
    }

    pub fn unchecked_send_home_action(&self, action: HomeAction) {
        self.send_home_action(action)
            .expect("failed to send home action")
    }

    pub fn unchecked_send(&self, action: Action) {
        self.send(action).expect("failed to send action")
    }
}
//
// impl<'de> Deserialize<'de> for Action {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         struct ActionVisitor;
//
//         impl<'de> Visitor<'de> for ActionVisitor {
//             type Value = Action;
//
//             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//                 formatter.write_str("a valid string representation of Action")
//             }
//
//             fn visit_str<E>(self, value: &str) -> Result<Action, E>
//             where
//                 E: de::Error,
//             {
//                 match value {
//                     "Tick" => Ok(Action::Tick),
//                     "Quit" => Ok(Action::Quit),
//                     "ScheduleContractRefresh" => Ok(Action::ScheduleContractRefresh),
//                     "ToggleShowHelp" => Ok(Action::ToggleShowHelp),
//                     // "ProcessInput" => Ok(Action::ProcessInput),
//                     "EnterNormal" => Ok(Action::EnterNormal),
//                     data if data.starts_with("Error(") => {
//                         let error_msg = data.trim_start_matches("Error(").trim_end_matches(")");
//                         Ok(Action::Error(error_msg.to_string()))
//                     }
//                     data if data.starts_with("Resize(") => {
//                         let parts: Vec<&str> = data
//                             .trim_start_matches("Resize(")
//                             .trim_end_matches(")")
//                             .split(',')
//                             .collect();
//                         if parts.len() == 2 {
//                             let width: u16 = parts[0].trim().parse().map_err(E::custom)?;
//                             let height: u16 = parts[1].trim().parse().map_err(E::custom)?;
//                             Ok(Action::Resize(width, height))
//                         } else {
//                             Err(E::custom(format!("Invalid Resize format: {}", value)))
//                         }
//                     }
//                     _ => Err(E::custom(format!("Unknown Action variant: {}", value))),
//                 }
//             }
//         }
//
//         deserializer.deserialize_str(ActionVisitor)
//     }
// }
