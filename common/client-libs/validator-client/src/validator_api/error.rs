use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorAPIError {
    #[error("There was an issue with the REST request - {source}")]
    ReqwestClientError {
        #[from]
        source: reqwest::Error,
    },
}
