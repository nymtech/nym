use semver::Version;
use semver::VersionReq;

/// Checks whether given `version` is compatible with a given semantic version requirement `req`
/// according to major-minor semver rules. The semantic version requirement can be passed as a full,
/// concrete version number, because that's what we'll have in our Cargo.toml files (e.g. 0.3.2).
/// The patch number in the requirement gets dropped and replaced with a wildcard (0.3.*) as all
/// minor versions should be compatible with each other.
pub fn is_compatible(version: &str, req: &str) -> bool {
    let version = Version::parse(version).unwrap();
    let tmp = Version::parse(req).unwrap();
    let wildcard = format!("{}.{}.*", tmp.major, tmp.minor).to_string();
    let semver_requirement = VersionReq::parse(&wildcard).unwrap();

    semver_requirement.matches(&version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_0_3_0_is_compatible_with_requirement_0_3_x() {
        assert!(is_compatible("0.3.0", "0.3.2"));
    }

    #[test]
    fn version_0_3_1_is_compatible_with_minimum_requirement_0_3_x() {
        assert!(is_compatible("0.3.1", "0.3.2"));
    }

    #[test]
    fn version_0_3_2_is_compatible_with_minimum_requirement_0_3_x() {
        assert!(is_compatible("0.3.2", "0.3.0"));
    }

    #[test]
    fn version_0_2_0_is_not_compatible_with_requirement_0_3_x() {
        assert!(!is_compatible("0.2.0", "0.3.2"));
    }

    #[test]
    fn version_0_4_0_is_not_compatible_with_requirement_0_3_x() {
        assert!(!is_compatible("0.4.0", "0.3.2"));
    }

    #[test]
    fn version_1_3_2_is_not_compatible_with_requirement_0_3_x() {
        assert!(!is_compatible("1.3.2", "0.3.2"));
    }
}
