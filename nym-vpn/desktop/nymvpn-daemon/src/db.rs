use nymvpn_config::config;
use nymvpn_migration::{
    sea_orm::{ConnectOptions, Database, DatabaseConnection},
    Migrator, MigratorTrait,
};

#[derive(Debug, Clone)]
pub struct Db {
    connection: DatabaseConnection,
}

impl Db {
    pub async fn new() -> Result<Self, String> {
        let config = config();
        tokio::fs::create_dir_all(config.db_dir())
            .await
            .map_err(|e| format!("Error creating DB directory {e}"))?;

        let mut opts = ConnectOptions::new(config.db_url());
        opts.sqlx_logging(false);
        Ok(Self {
            connection: Database::connect(opts)
                .await
                .map_err(|e| format!("Error connecting to database {e}"))?,
        })
    }

    pub async fn migrate(&self) -> Result<(), String> {
        Migrator::up(&self.connection, None)
            .await
            .map_err(|e| format!("failed to run db migration {e}"))
    }

    pub fn connection(&self) -> DatabaseConnection {
        self.connection.clone()
    }
}
