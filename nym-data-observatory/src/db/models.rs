use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::background_task::Response;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub(crate) struct JokeDto {
    pub(crate) joke_id: String,
    pub(crate) joke: String,
    pub(crate) date_created: i32,
}

impl From<Response> for JokeDto {
    fn from(value: Response) -> Self {
        Self {
            joke_id: value.joke_id,
            joke: value.joke,
            // casting not smart, can implicitly panic, don't do this in prod
            date_created: chrono::offset::Utc::now().timestamp() as i32,
        }
    }
}
