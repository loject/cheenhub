#![warn(missing_docs)]
//! Database migrations for CheenHub.

pub use sea_orm_migration::prelude::*;

/// Registry for CheenHub database migrations.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        Vec::new()
    }
}
