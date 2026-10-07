//! Переносит исходное время принятия соглашения в журнал юридических подтверждений.

use chrono::{DateTime, Utc};
use sea_orm_migration::prelude::*;
use uuid::Uuid;

const LEGACY_ACCEPTANCE_SOURCE: &str = "legacy_users.accepted_terms_at";
const LEGACY_INSERT_BATCH_SIZE: usize = 500;

/// Миграция сохранения исходного времени принятия пользовательского соглашения.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LegalAcceptances::Table)
                    .modify_column(
                        ColumnDef::new(LegalAcceptances::DocumentVersion)
                            .string_len(32)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_legal_acceptances_unknown_version")
                    .table(LegalAcceptances::Table)
                    .col(LegalAcceptances::UserId)
                    .col(LegalAcceptances::DocumentKind)
                    .unique()
                    .and_where(Expr::col(LegalAcceptances::DocumentVersion).is_null())
                    .to_owned(),
            )
            .await?;

        let legacy_acceptances = manager
            .get_connection()
            .query_all(&legacy_acceptances_to_insert())
            .await?
            .into_iter()
            .map(|row| {
                Ok(LegacyAcceptance {
                    user_id: row.try_get("", "id")?,
                    accepted_at: row.try_get("", "accepted_terms_at")?,
                })
            })
            .collect::<Result<Vec<_>, DbErr>>()?;

        for batch in legacy_acceptances.chunks(LEGACY_INSERT_BATCH_SIZE) {
            manager.execute(insert_legacy_acceptances(batch)).await?;
        }

        tracing::info!(
            migrated_acceptances = legacy_acceptances.len(),
            "перенесены исходные даты принятия пользовательского соглашения"
        );

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::AcceptedTermsAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::AcceptedTermsAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager.execute(restore_legacy_acceptance_dates()).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(
                        ColumnDef::new(Users::AcceptedTermsAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager.execute(delete_legacy_acceptances()).await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_legal_acceptances_unknown_version")
                    .table(LegalAcceptances::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(LegalAcceptances::Table)
                    .modify_column(
                        ColumnDef::new(LegalAcceptances::DocumentVersion)
                            .string_len(32)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }
}

struct LegacyAcceptance {
    user_id: Uuid,
    accepted_at: DateTime<Utc>,
}

fn legacy_acceptances_to_insert() -> SelectStatement {
    let existing_marker = Query::select()
        .expr(Expr::val(1))
        .from(LegalAcceptances::Table)
        .and_where(Expr::col(LegalAcceptances::UserId).eq(Expr::col((Users::Table, Users::Id))))
        .and_where(Expr::col(LegalAcceptances::DocumentKind).eq("terms"))
        .and_where(Expr::col(LegalAcceptances::AcceptanceSource).eq(LEGACY_ACCEPTANCE_SOURCE))
        .and_where(Expr::col(LegalAcceptances::DocumentVersion).is_null())
        .take();

    Query::select()
        .column(Users::Id)
        .column(Users::AcceptedTermsAt)
        .from(Users::Table)
        .and_where(Expr::not_exists(existing_marker))
        .to_owned()
}

fn insert_legacy_acceptances(batch: &[LegacyAcceptance]) -> InsertStatement {
    let mut insert = Query::insert();
    insert.into_table(LegalAcceptances::Table).columns([
        LegalAcceptances::Id,
        LegalAcceptances::UserId,
        LegalAcceptances::DocumentKind,
        LegalAcceptances::DocumentVersion,
        LegalAcceptances::AcceptanceSource,
        LegalAcceptances::AcceptedAt,
    ]);

    for acceptance in batch {
        insert.values_panic([
            Uuid::new_v4().into(),
            acceptance.user_id.into(),
            "terms".into(),
            Option::<String>::None.into(),
            LEGACY_ACCEPTANCE_SOURCE.into(),
            acceptance.accepted_at.into(),
        ]);
    }

    insert.take()
}

fn restore_legacy_acceptance_dates() -> UpdateStatement {
    let legacy_timestamp = Query::select()
        .column(LegalAcceptances::AcceptedAt)
        .from(LegalAcceptances::Table)
        .and_where(Expr::col(LegalAcceptances::UserId).eq(Expr::col((Users::Table, Users::Id))))
        .and_where(Expr::col(LegalAcceptances::DocumentKind).eq("terms"))
        .and_where(Expr::col(LegalAcceptances::AcceptanceSource).eq(LEGACY_ACCEPTANCE_SOURCE))
        .and_where(Expr::col(LegalAcceptances::DocumentVersion).is_null())
        .limit(1)
        .take();
    let earliest_terms_timestamp = Query::select()
        .expr(Func::min(Expr::col(LegalAcceptances::AcceptedAt)))
        .from(LegalAcceptances::Table)
        .and_where(Expr::col(LegalAcceptances::UserId).eq(Expr::col((Users::Table, Users::Id))))
        .and_where(Expr::col(LegalAcceptances::DocumentKind).eq("terms"))
        .take();

    Query::update()
        .table(Users::Table)
        .value(
            Users::AcceptedTermsAt,
            Func::coalesce([
                Expr::from(legacy_timestamp),
                Expr::from(earliest_terms_timestamp),
            ]),
        )
        .to_owned()
}

fn delete_legacy_acceptances() -> DeleteStatement {
    Query::delete()
        .from_table(LegalAcceptances::Table)
        .cond_where(
            Cond::all()
                .add(Expr::col(LegalAcceptances::DocumentKind).eq("terms"))
                .add(Expr::col(LegalAcceptances::DocumentVersion).is_null())
                .add(Expr::col(LegalAcceptances::AcceptanceSource).eq(LEGACY_ACCEPTANCE_SOURCE)),
        )
        .to_owned()
}

#[derive(DeriveIden)]
enum LegalAcceptances {
    Table,
    Id,
    UserId,
    DocumentKind,
    DocumentVersion,
    AcceptanceSource,
    AcceptedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    AcceptedTermsAt,
}
