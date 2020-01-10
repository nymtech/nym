use log::trace;
use sphinx::route::Node as SphinxNode;

#[derive(Debug)]
pub(crate) enum PathCheckerError {
    CouldNotRegisterWithEndProviderError,
}

pub(crate) struct PathChecker {}

impl PathChecker {
    pub(crate) fn new() -> Result<Self, PathCheckerError> {
        Ok(PathChecker {})
    }

    pub(crate) fn check_path(&self, path: &Vec<SphinxNode>) -> bool {
        trace!("Checking path: {:?}", path);

        // TODO:
        true
    }
}
