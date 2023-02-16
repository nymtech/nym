// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::raw_msg_to_string;
use cosmwasm_std::{Addr, BankMsg, Binary, Coin, Event};

fn format_coins(coins: &[Coin]) -> String {
    if coins.is_empty() {
        "<zero>".to_string()
    } else {
        coins
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }
}

// specifically for tokens included in `Execute` that go into the contract
#[derive(Debug, Clone)]
pub struct CrossContractTokenMove {
    pub amount: Vec<Coin>,
    pub sender: Addr,
    pub receiver: Addr,
}

impl CrossContractTokenMove {
    pub fn new(amount: Vec<Coin>, sender: Addr, receiver: Addr) -> Self {
        Self {
            amount,
            sender,
            receiver,
        }
    }

    pub fn pretty(&self) -> String {
        let total_amount = format_coins(&self.amount);

        format!(
            "{total_amount} will be transferred from {} to {} (CONTRACTS)",
            self.sender, self.receiver
        )
    }
}

#[derive(Debug, Default)]
pub struct ExecutionResult {
    pub steps: Vec<ExecutionStepResult>,
}

impl ExecutionResult {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn pretty(&self) -> String {
        let mut out = String::new();
        for (i, step) in self.steps.iter().enumerate() {
            out.push_str(&format!("STEP {}\n", i + 1));
            out.push_str(&format!("{}\n", step.pretty()));
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct FurtherExecution {
    pub contract: Addr,
    pub msg: Binary,
    pub funds: Vec<Coin>,
}

impl FurtherExecution {
    pub fn new(contract: String, msg: Binary, funds: Vec<Coin>) -> Self {
        Self {
            contract: Addr::unchecked(contract),
            msg,
            funds,
        }
    }

    pub fn pretty(&self) -> String {
        let msg = raw_msg_to_string(&self.msg);
        let total_funds = format_coins(&self.funds);

        format!(
            "{} will be called with msg {msg} and {total_funds} funds",
            self.contract
        )
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionStepResult {
    pub events: Vec<Event>,
    pub incoming_tokens: Vec<CrossContractTokenMove>,
    pub bank_msgs: Vec<BankMsg>,
    pub further_execution: Vec<FurtherExecution>,
}

impl ExecutionStepResult {
    pub fn pretty(&self) -> String {
        let mut out = String::new();

        // let's keep them squished for now...
        let events = format!("EVENTS: {:?}\n", self.events);
        out.push_str(&events);

        if self.incoming_tokens.iter().any(|c| !c.amount.is_empty()) {
            out.push_str("MOVED TOKENS (CONTRACTS):\n");
            for incoming in &self.incoming_tokens {
                if !incoming.amount.is_empty() {
                    out.push_str(&format!("{}\n", incoming.pretty()))
                }
            }
        }

        if !self.bank_msgs.is_empty() {
            out.push_str("MOVED TOKENS (BANK):\n");
            for bank in &self.bank_msgs {
                let formatted = match bank {
                    BankMsg::Send { to_address, amount } => format!(
                        "{} will be transferred to {to_address}",
                        format_coins(amount)
                    ),
                    BankMsg::Burn { amount } => format!("{} WILL BE BURNT", format_coins(amount)),
                    _ => "unknown variant of BankMsg was introduced!".to_string(),
                };
                out.push_str(&format!("{formatted}\n"))
            }
        }

        if !self.further_execution.is_empty() {
            out.push_str("FURTHER CONTRACT CALLS:\n");
            for further_exec in &self.further_execution {
                out.push_str(&format!("{}\n", further_exec.pretty()))
            }
        }

        out
    }
}
