use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Device::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Device::UniqueId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Device::Name).string().not_null())
                    .col(ColumnDef::new(Device::Version).string().not_null())
                    .col(ColumnDef::new(Device::Arch).string().not_null())
                    .col(ColumnDef::new(Device::DeviceType).string().not_null())
                    .col(ColumnDef::new(Device::PrivateKey).string().not_null())
                    .col(ColumnDef::new(Device::Ipv4Address).string())
                    .col(ColumnDef::new(Device::CreatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Device::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Device {
    Table,
    Name,
    Version,
    Arch,
    UniqueId,
    DeviceType,
    PrivateKey,
    Ipv4Address,
    CreatedAt,
}
