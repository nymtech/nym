use chrono::Utc;

#[allow(clippy::module_name_repetitions)]
pub struct EphemeraTime;

impl EphemeraTime {
    #[allow(clippy::cast_sign_loss)]
    pub fn now() -> u64 {
        Utc::now().timestamp_millis() as u64
    }
}
