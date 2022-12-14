use nym_api_requests::models::RequestError;
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

    #[error("The validator API has failed to resolve our request. It returned status code {status} and additional error message: {}", error.message())]
    ApiRequestFailure { status: u16, error: RequestError },
}
