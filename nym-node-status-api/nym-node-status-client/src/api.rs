use std::fmt::Display;

pub(super) struct ApiPaths {
    server_address: String,
}

impl ApiPaths {
    pub(super) fn new(server_address: String) -> Self {
        Self { server_address }
    }
    pub(super) fn request_testrun(&self) -> String {
        format!("{}/internal/testruns", self.server_address)
    }

    pub(super) fn submit_results(&self, testrun_id: impl Display) -> String {
        format!("{}/internal/testruns/{}", self.server_address, testrun_id)
    }

    pub(super) fn submit_results_v2(&self, testrun_id: impl Display) -> String {
        format!(
            "{}/internal/testruns/{}/v2",
            self.server_address, testrun_id
        )
    }
}
