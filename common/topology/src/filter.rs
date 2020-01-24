pub trait Versioned: Clone {
    fn get_version(&self) -> String;
}

pub trait VersionFiltererable<T> {
    fn filter_by_version(&self, expected_version: &str) -> Self;
}

impl<T: Versioned> VersionFiltererable<T> for Vec<T> {
    fn filter_by_version(&self, expected_version: &str) -> Self {
        self.iter()
            .filter(|node| version_checker::is_compatible(&node.get_version(), expected_version))
            .cloned()
            .collect()
    }
}
