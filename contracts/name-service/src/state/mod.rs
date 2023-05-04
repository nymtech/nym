pub mod admin;
pub mod config;
pub mod name_id_counter;
pub mod names;

pub(crate) use admin::{assert_admin, set_admin};
pub(crate) use config::{deposit_required, load_config, save_config, Config};
pub(crate) use name_id_counter::next_name_id_counter;
