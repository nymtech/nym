// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) fn parse_rotation_id_from_filename(name: &str) -> Option<u32> {
    let stripped = name.strip_prefix("rot-")?;
    let ext_idx = stripped.rfind(".").unwrap_or(stripped.len());
    let rotation = stripped.chars().take(ext_idx).collect::<String>();
    rotation.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_rotation_id() {
        let test_cases = vec![
            ("rot", None),
            ("rot-123", Some(123)),
            ("foo-123", None),
            ("rot-123.ext", Some(123)),
            ("rot-123.different-ext", Some(123)),
            ("rot.123.aaa", None),
        ];

        for (raw, expected) in test_cases {
            assert_eq!(
                parse_rotation_id_from_filename(raw),
                expected,
                "failed: {raw} to {expected:?}"
            );
        }
    }
}
