use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Asset::Table)
                    .rename_column(Asset::IssuedSupply, Asset::InitialSupply)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Asset::Table)
                    .add_column(ColumnDef::new(Asset::MaxSupply).string().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Asset::Table)
                    .add_column(
                        ColumnDef::new(Asset::KnownCirculatingSupply)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Asset::Table)
                    .rename_column(Asset::InitialSupply, Asset::IssuedSupply)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Asset::Table)
                    .drop_column(Asset::MaxSupply)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Asset::Table)
                    .drop_column(Asset::KnownCirculatingSupply)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum Asset {
    Table,
    IssuedSupply,
    InitialSupply,
    MaxSupply,
    KnownCirculatingSupply,
}
