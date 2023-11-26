// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::DkgState;
use nym_coconut_dkg_common::dealer::{ContractDealing, DealerDetails};
use nym_coconut_dkg_common::types::Epoch;
use nym_validator_client::nyxd::{cw4, cw_controllers};
use serde::{
    de::{self, Deserializer, Visitor},
    Deserialize, Serialize,
};
use std::fmt;

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
    Tick,
    Render,
    Resize(u16, u16),
    Quit,
    Refresh,
    Error(String),
    Help,
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
    Update,
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ActionVisitor;

        impl<'de> Visitor<'de> for ActionVisitor {
            type Value = Action;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid string representation of Action")
            }

            fn visit_str<E>(self, value: &str) -> Result<Action, E>
            where
                E: de::Error,
            {
                match value {
                    "Tick" => Ok(Action::Tick),
                    "Render" => Ok(Action::Render),
                    "Quit" => Ok(Action::Quit),
                    "Refresh" => Ok(Action::Refresh),
                    "Help" => Ok(Action::Help),
                    "ScheduleContractRefresh" => Ok(Action::ScheduleContractRefresh),
                    "ToggleShowHelp" => Ok(Action::ToggleShowHelp),
                    // "ProcessInput" => Ok(Action::ProcessInput),
                    "EnterNormal" => Ok(Action::EnterNormal),
                    data if data.starts_with("Error(") => {
                        let error_msg = data.trim_start_matches("Error(").trim_end_matches(")");
                        Ok(Action::Error(error_msg.to_string()))
                    }
                    data if data.starts_with("Resize(") => {
                        let parts: Vec<&str> = data
                            .trim_start_matches("Resize(")
                            .trim_end_matches(")")
                            .split(',')
                            .collect();
                        if parts.len() == 2 {
                            let width: u16 = parts[0].trim().parse().map_err(E::custom)?;
                            let height: u16 = parts[1].trim().parse().map_err(E::custom)?;
                            Ok(Action::Resize(width, height))
                        } else {
                            Err(E::custom(format!("Invalid Resize format: {}", value)))
                        }
                    }
                    _ => Err(E::custom(format!("Unknown Action variant: {}", value))),
                }
            }
        }

        deserializer.deserialize_str(ActionVisitor)
    }
}
