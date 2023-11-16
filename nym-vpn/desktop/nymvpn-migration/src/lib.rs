pub use sea_orm_migration::prelude::*;

mod m20230603_133547_create_vpn_session;
mod m20230603_133704_create_device;
mod m20230603_133805_create_token;
mod m20230603_133832_create_recent_locations;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230603_133547_create_vpn_session::Migration),
            Box::new(m20230603_133704_create_device::Migration),
            Box::new(m20230603_133805_create_token::Migration),
            Box::new(m20230603_133832_create_recent_locations::Migration),
        ]
    }
}
