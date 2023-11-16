use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RecentLocations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecentLocations::Id)
                            .auto_increment()
                            .not_null()
                            .integer()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RecentLocations::Code).string().not_null())
                    .col(ColumnDef::new(RecentLocations::City).string().not_null())
                    .col(
                        ColumnDef::new(RecentLocations::CityCode)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RecentLocations::Country).string().not_null())
                    .col(
                        ColumnDef::new(RecentLocations::CountryCode)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RecentLocations::State).string())
                    .col(ColumnDef::new(RecentLocations::StateCode).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecentLocations::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum RecentLocations {
    Table,
    Id,
    Code,
    City,
    CityCode,
    Country,
    CountryCode,
    State,
    StateCode,
}
