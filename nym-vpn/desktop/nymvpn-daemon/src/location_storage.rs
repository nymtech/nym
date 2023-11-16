use nymvpn_migration::{
    sea_orm::{
        ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
        QuerySelect, QueryTrait, Set,
    },
    DbErr,
};
use nymvpn_types::location::Location;

const RECENT_LIMIT: u64 = 2;

#[derive(Clone)]
pub struct LocationStorage {
    db: DatabaseConnection,
}

impl LocationStorage {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn add_recent(&self, location: Location) -> Result<(), DbErr> {
        let _ = nymvpn_entity::recent_locations::Entity::delete_many()
            .filter(nymvpn_entity::recent_locations::Column::Code.eq(location.code.clone()))
            .exec(&self.db)
            .await?;

        // insert new record
        let recent_location = nymvpn_entity::recent_locations::ActiveModel {
            code: Set(location.code),
            city: Set(location.city),
            city_code: Set(location.city_code),
            country: Set(location.country),
            country_code: Set(location.country_code),
            state: Set(location.state),
            state_code: Set(location.state_code),
            ..Default::default()
        };

        recent_location.insert(&self.db).await?;

        // delete older ones
        let deleted_result = nymvpn_entity::recent_locations::Entity::delete_many()
            .filter(
                nymvpn_entity::recent_locations::Column::Id.not_in_subquery(
                    nymvpn_entity::recent_locations::Entity::find()
                        .select_only()
                        .column(nymvpn_entity::recent_locations::Column::Id)
                        .order_by_desc(nymvpn_entity::recent_locations::Column::Id)
                        .limit(RECENT_LIMIT)
                        .into_query(),
                ),
            )
            .exec(&self.db)
            .await?;

        tracing::debug!(
            "deleted from recent locations {}",
            deleted_result.rows_affected
        );

        Ok(())
    }

    pub async fn recent(&self) -> Result<Vec<Location>, DbErr> {
        Ok(nymvpn_entity::recent_locations::Entity::find()
            .distinct()
            .order_by_desc(nymvpn_entity::recent_locations::Column::Id)
            .group_by(nymvpn_entity::recent_locations::Column::Code)
            .all(&self.db)
            .await?
            .into_iter()
            .map(nymvpn_types::location::Location::from)
            .collect())
    }
}
