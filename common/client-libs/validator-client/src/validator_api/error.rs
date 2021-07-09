use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorAPIClientError {
    #[error("There was an issue with the REST request - {source}")]
    ReqwestClientError {
        #[from]
        source: reqwest::Error,
    },
}
