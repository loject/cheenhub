//! Конкретные блокировки операций жизненного цикла аккаунта.
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait,
    sea_query::{Alias, Expr, Func, Query},
};
use uuid::Uuid;

/// Удерживает блокировку до завершения прикладной операции.
pub(crate) enum AccountLifecycleGuard {
    /// Транзакционная блокировка PostgreSQL; rollback при освобождении.
    Postgres {
        /// Транзакция, удерживающая advisory lock.
        _transaction: DatabaseTransaction,
    },
    /// Общая блокировка тестового хранилища.
    InMemory {
        /// Владение блокировкой до завершения операции.
        _guard: tokio::sync::OwnedMutexGuard<()>,
    },
}

pub(super) async fn lock_postgres(
    database: &DatabaseConnection,
    user_id: &Uuid,
) -> anyhow::Result<AccountLifecycleGuard> {
    let mut key = [0_u8; 8];
    key.copy_from_slice(&user_id.as_bytes()[..8]);
    let query = Query::select()
        .expr_as(
            Func::cust(Alias::new("pg_try_advisory_xact_lock"))
                .arg(Expr::val(i64::from_be_bytes(key))),
            Alias::new("acquired"),
        )
        .to_owned();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let transaction = database.begin().await?;
        let row = transaction
            .query_one(transaction.get_database_backend().build(&query))
            .await?
            .ok_or_else(|| anyhow::anyhow!("account lifecycle lock returned no result"))?;
        if row.try_get::<bool>("", "acquired")? {
            return Ok(AccountLifecycleGuard::Postgres {
                _transaction: transaction,
            });
        }
        transaction.rollback().await?;
        if tokio::time::Instant::now() >= deadline {
            tracing::warn!(%user_id, "account lifecycle lock timed out");
            anyhow::bail!("account lifecycle operation is busy");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
