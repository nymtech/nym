use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VpnSession::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VpnSession::RequestId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(VpnSession::LocationCode).string().not_null())
                    .col(ColumnDef::new(VpnSession::LocationCity).string().not_null())
                    .col(
                        ColumnDef::new(VpnSession::LocationCityCode)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VpnSession::LocationCountry)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VpnSession::LocationCountryCode)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(VpnSession::LocationState).string())
                    .col(ColumnDef::new(VpnSession::LocationStateCode).string())
                    .col(ColumnDef::new(VpnSession::ServerStatus).string())
                    .col(ColumnDef::new(VpnSession::SessionUuid).string())
                    .col(ColumnDef::new(VpnSession::ServerIpv4Endpoint).string())
                    .col(ColumnDef::new(VpnSession::ServerPrivateIpv4).string())
                    .col(ColumnDef::new(VpnSession::ServerPublicKey).string())
                    .col(
                        ColumnDef::new(VpnSession::RequestedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VpnSession::MarkForDeletion)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(VpnSession::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum VpnSession {
    Table,
    RequestId,
    LocationCode,
    LocationCity,
    LocationCityCode,
    LocationCountry,
    LocationCountryCode,
    LocationState,
    LocationStateCode,
    ServerStatus,
    SessionUuid,
    ServerIpv4Endpoint,
    ServerPrivateIpv4,
    ServerPublicKey,
    RequestedAt,
    MarkForDeletion,
}
