#![warn(missing_docs)]
//! Migration CLI entrypoint for CheenHub.

use sea_orm_migration::cli;

#[tokio::main]
async fn main() {
    cli::run_cli(cheenhub_migrations::Migrator).await;
}
