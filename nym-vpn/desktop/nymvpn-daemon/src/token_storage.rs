use nymvpn_entity::token::Entity as Token;
use nymvpn_migration::{
    sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set},
    DbErr,
};
use nymvpn_server::auth::TokenProvider;

#[derive(Debug, Clone)]
pub struct TokenStorage {
    db: DatabaseConnection,
}

impl TokenStorage {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_token(&self) -> Result<Option<String>, DbErr> {
        let token = Token::find().one(&self.db).await?;

        Ok(token.map(|t| t.token))
    }

    pub async fn save_token(&self, token: String) -> Result<(), DbErr> {
        let token = nymvpn_entity::token::ActiveModel {
            token: Set(token),
            ..Default::default()
        };

        let new_token = token.insert(&self.db).await?;

        // delete previous tokens
        let deleted = Token::delete_many()
            .filter(nymvpn_entity::token::Column::Id.lt(new_token.id))
            .exec(&self.db)
            .await?;

        tracing::info!(
            "new token saved; deleted old tokens #{}",
            deleted.rows_affected
        );

        Ok(())
    }

    pub async fn remove_all(&self) -> Result<(), DbErr> {
        let deleted = Token::delete_many().exec(&self.db).await?;

        tracing::info!("deleted tokens #{}", deleted.rows_affected);

        Ok(())
    }
}

#[async_trait::async_trait]
impl TokenProvider for TokenStorage {
    async fn bearer_token(&self) -> Option<String> {
        let token = self
            .get_token()
            .await
            .map_err(|e| tracing::error!("failed to get token from db: {e}"));

        match token {
            Ok(token) => token,
            Err(_) => None,
        }
    }
}
