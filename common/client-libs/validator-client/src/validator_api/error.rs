use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorAPIError {
    #[error("There was an issue with the REST request - {source}")]
    ReqwestClientError {
        #[from]
        source: reqwest::Error,
    },

    #[error("Request failed with error message - {0}")]
    GenericRequestFailure(String),
}
