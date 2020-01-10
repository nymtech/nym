use log::trace;
use sphinx::route::Node as SphinxNode;

pub(crate) struct PathChecker {}

impl PathChecker {
    pub(crate) fn new() -> Self {
        PathChecker {}
    }

    pub(crate) fn check_path(&self, path: &Vec<SphinxNode>) -> bool {
        trace!("Checking path: {:?}", path);

        // TODO:
        true
    }
}
