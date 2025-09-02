// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use itertools::Itertools;

fn maybe_bool_to_emoji_string(maybe_bool: Option<bool>) -> String {
    match maybe_bool {
        None => "⚠️ unknown".into(),
        Some(true) => "✅ yes".into(),
        Some(false) => "❌ no".into(),
    }
}

pub(crate) struct DisplayableSignerResult {
    pub(crate) url: String,
    pub(crate) version: Option<String>,
    pub(crate) signing_available: Option<bool>,
    pub(crate) chain_not_stalled: Option<bool>,
}

impl DisplayableSignerResult {
    fn to_markdown_table_row(&self) -> String {
        format!(
            "| {} | {} | {} | {} |",
            self.url,
            self.version.as_deref().unwrap_or("unknown"),
            maybe_bool_to_emoji_string(self.signing_available),
            maybe_bool_to_emoji_string(self.chain_not_stalled)
        )
    }
}

pub(crate) struct TestResult {
    pub(crate) summary: Summary,
    pub(crate) signers: Vec<DisplayableSignerResult>,
}

impl TestResult {
    pub(crate) fn quorum_unavailable(&self) -> bool {
        self.summary.signing_quorum_available.unwrap_or(false)
    }

    pub(crate) fn quorum_unknown(&self) -> bool {
        self.summary.signing_quorum_available.is_none()
    }

    pub(crate) fn results_to_markdown_message(&self) -> String {
        let p_available = format!(
            "{:.2}",
            (self.summary.fully_working as f32 / self.summary.registered_signers as f32) * 100.
        );

        format!(
            r#"
## Summary
- quorum available: {}   ({p_available}% of signers fully available)
- signers fully working: {}
- signing threshold: {}
- registered signers: {}
- unreachable signers: {}

### Chain Status
- unknown status: {}
- working chain: {}
- stalled chain: {}

### Credential Issuance Status
(note: signers below 1.1.64 do not return fully reliable results)
- unknown status: {}
- working issuance: {}
- unavailable issuance: {}

## Detailed Results
| address | version | chain working | issuance (maybe) available |
| - | - | - | - |
{}
        "#,
            maybe_bool_to_emoji_string(self.summary.signing_quorum_available),
            self.summary.fully_working,
            self.summary
                .threshold
                .map(|threshold| threshold.to_string())
                .unwrap_or("???".to_string()),
            self.summary.registered_signers,
            self.summary.unreachable_signers,
            self.summary.unknown_local_chain_status,
            self.summary.working_local_chain,
            self.summary.stalled_local_chain,
            self.summary.unknown_credential_issuance_status,
            self.summary.working_credential_issuance,
            self.summary.unavailable_credential_issuance,
            self.signers
                .iter()
                .map(|r| r.to_markdown_table_row())
                .join("\n")
        )
    }
}

pub(crate) struct Summary {
    pub(crate) signing_quorum_available: Option<bool>,
    pub(crate) fully_working: usize,
    pub(crate) threshold: Option<u64>,

    pub(crate) registered_signers: usize,
    pub(crate) unreachable_signers: usize,

    pub(crate) unknown_local_chain_status: usize,
    pub(crate) stalled_local_chain: usize,
    pub(crate) working_local_chain: usize,

    pub(crate) unknown_credential_issuance_status: usize,
    pub(crate) working_credential_issuance: usize,
    pub(crate) unavailable_credential_issuance: usize,
}
