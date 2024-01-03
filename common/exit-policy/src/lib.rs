// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod policy;

#[cfg(feature = "client")]
pub mod client;

pub use crate::policy::{
    AddressPolicy, AddressPolicyAction, AddressPolicyRule, AddressPortPattern, PolicyError,
    PortRange,
};

pub(crate) const EXIT_POLICY_FIELD_NAME: &str = "ExitPolicy";
const COMMENT_CHAR: char = '#';

pub type ExitPolicy = AddressPolicy;

pub fn parse_exit_policy<S: AsRef<str>>(exit_policy: S) -> Result<ExitPolicy, PolicyError> {
    let rules = exit_policy
        .as_ref()
        .lines()
        .map(|maybe_rule| {
            if let Some(comment_start) = maybe_rule.find(COMMENT_CHAR) {
                &maybe_rule[..comment_start]
            } else {
                maybe_rule
            }
            .trim()
        })
        .filter(|maybe_rule| !maybe_rule.is_empty())
        .map(parse_address_policy_rule)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AddressPolicy { rules })
}

pub fn format_exit_policy(policy: &ExitPolicy) -> String {
    policy
        .rules
        .iter()
        .map(|rule| format!("{EXIT_POLICY_FIELD_NAME} {rule}"))
        .fold(String::new(), |accumulator, rule| {
            accumulator + &rule + "\n"
        })
        .trim_end()
        .to_string()
}

fn parse_address_policy_rule(rule: &str) -> Result<AddressPolicyRule, PolicyError> {
    // each exit policy rule must begin with 'ExitPolicy' followed by the actual rule
    rule.strip_prefix(EXIT_POLICY_FIELD_NAME)
        .ok_or(PolicyError::NoExitPolicyPrefix {
            entry: rule.to_string(),
        })?
        .trim()
        .parse()
}

// for each line, ignore everything after the comment

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::AddressPolicyAction::{Accept, Accept6, Reject, Reject6};
    use crate::policy::{AddressPortPattern, IpPattern, PortRange};

    #[test]
    fn parsing_policy() {
        let sample = r#"
ExitPolicy reject 1.2.3.4/32:*#comment
ExitPolicy reject 1.2.3.5:* #comment
ExitPolicy reject 1.2.3.6/16:*
ExitPolicy reject 1.2.3.6/16:123-456 # comment

ExitPolicy accept *:53 # DNS

# random comment

ExitPolicy accept6 *6:119
ExitPolicy accept *4:120
ExitPolicy reject6 [FC00::]/7:*

# Portless
ExitPolicy accept *:0
ExitPolicy accept *4:0
ExitPolicy accept *6:0

ExitPolicy reject *:0
ExitPolicy reject *4:0
ExitPolicy reject *6:0

#ExitPolicy accept *:8080 #and another comment here

ExitPolicy reject FE80:0000:0000:0000:0202:B3FF:FE1E:8329:*
ExitPolicy reject FE80:0000:0000:0000:0202:B3FF:FE1E:8328:1234
ExitPolicy reject FE80:0000:0000:0000:0202:B3FF:FE1E:8328/64:1235

#another comment
#ExitPolicy accept *:8080 

ExitPolicy reject *:*
        "#;

        let res = parse_exit_policy(sample).unwrap();

        let mut expected = AddressPolicy::new();

        // ExitPolicy reject 1.2.3.4/32:*#comment
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V4 {
                    addr_prefix: "1.2.3.4".parse().unwrap(),
                    mask: 32,
                },
                ports: PortRange::new_all(),
            },
        );

        // ExitPolicy reject 1.2.3.5:* #comment
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V4 {
                    addr_prefix: "1.2.3.5".parse().unwrap(),
                    mask: 32,
                },
                ports: PortRange::new_all(),
            },
        );

        // ExitPolicy reject 1.2.3.6/16:*
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V4 {
                    addr_prefix: "1.2.3.6".parse().unwrap(),
                    mask: 16,
                },
                ports: PortRange::new_all(),
            },
        );

        // ExitPolicy reject 1.2.3.6/16:123-456
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V4 {
                    addr_prefix: "1.2.3.6".parse().unwrap(),
                    mask: 16,
                },
                ports: PortRange::new(123, 456).unwrap(),
            },
        );

        // ExitPolicy accept *:53
        expected.push(
            Accept,
            AddressPortPattern {
                ip_pattern: IpPattern::Star,
                ports: PortRange::new_singleton(53),
            },
        );

        // ExitPolicy accept6 *6:119
        expected.push(
            Accept6,
            AddressPortPattern {
                ip_pattern: IpPattern::V6Star,
                ports: PortRange::new_singleton(119),
            },
        );

        // ExitPolicy accept *4:120
        expected.push(
            Accept,
            AddressPortPattern {
                ip_pattern: IpPattern::V4Star,
                ports: PortRange::new_singleton(120),
            },
        );

        // ExitPolicy reject6 [FC00::]/7:*
        expected.push(
            Reject6,
            AddressPortPattern {
                ip_pattern: IpPattern::V6 {
                    addr_prefix: "FC00::".parse().unwrap(),
                    mask: 7,
                },
                ports: PortRange::new_all(),
            },
        );

        // ExitPolicy accept *:0
        expected.push(
            Accept,
            AddressPortPattern {
                ip_pattern: IpPattern::Star,
                ports: PortRange::new_zero(),
            },
        );

        // ExitPolicy accept *4:0
        expected.push(
            Accept,
            AddressPortPattern {
                ip_pattern: IpPattern::V4Star,
                ports: PortRange::new_zero(),
            },
        );

        // ExitPolicy accept *6:0
        expected.push(
            Accept,
            AddressPortPattern {
                ip_pattern: IpPattern::V6Star,
                ports: PortRange::new_zero(),
            },
        );

        // ExitPolicy reject *:0
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::Star,
                ports: PortRange::new_zero(),
            },
        );

        // ExitPolicy reject *4:0
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V4Star,
                ports: PortRange::new_zero(),
            },
        );

        // ExitPolicy reject *6:0
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V6Star,
                ports: PortRange::new_zero(),
            },
        );

        // ExitPolicy FE80:0000:0000:0000:0202:B3FF:FE1E:8329:*
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V6 {
                    addr_prefix: "FE80:0000:0000:0000:0202:B3FF:FE1E:8329".parse().unwrap(),
                    mask: 128,
                },
                ports: PortRange::new_all(),
            },
        );

        // ExitPolicy FE80:0000:0000:0000:0202:B3FF:FE1E:8328:1234
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V6 {
                    addr_prefix: "FE80:0000:0000:0000:0202:B3FF:FE1E:8328".parse().unwrap(),
                    mask: 128,
                },
                ports: PortRange::new_singleton(1234),
            },
        );

        // ExitPolicy FE80:0000:0000:0000:0202:B3FF:FE1E:8328/64:1235
        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::V6 {
                    addr_prefix: "FE80:0000:0000:0000:0202:B3FF:FE1E:8328".parse().unwrap(),
                    mask: 64,
                },
                ports: PortRange::new_singleton(1235),
            },
        );

        expected.push(
            Reject,
            AddressPortPattern {
                ip_pattern: IpPattern::Star,
                ports: PortRange::new_all(),
            },
        );

        assert_eq!(res, expected)
    }
}
