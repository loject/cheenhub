#![warn(missing_docs)]
//! Database migrations for CheenHub.

mod m20260505_000001_create_auth_tables;
mod m20260505_000002_create_servers_table;

pub use sea_orm_migration::prelude::*;

/// Registry for CheenHub database migrations.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260505_000001_create_auth_tables::Migration),
            Box::new(m20260505_000002_create_servers_table::Migration),
        ]
    }
}
