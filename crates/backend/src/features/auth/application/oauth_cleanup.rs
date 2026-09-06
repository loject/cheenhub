//! Периодическая очистка временных данных desktop OAuth в постоянном хранилище.

use chrono::Utc;
use std::{sync::Arc, time::Duration};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

use crate::features::auth::infrastructure::PostgresAuthStore;

/// Удаляет истёкшие попытки при запуске и затем раз в минуту, включая периоды без входов.
pub(crate) async fn run_desktop_oauth_cleanup(store: Arc<PostgresAuthStore>) {
    let mut ticker = interval(Duration::from_secs(60));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    info!(
        interval_seconds = 60,
        "Запущена очистка временных данных desktop OAuth"
    );
    loop {
        ticker.tick().await;
        // Один cutoff на проход: новые попытки не продлевают очистку накопившихся записей.
        let cutoff = Utc::now();
        let mut removed = 0;
        loop {
            match store.cleanup_desktop_oauth(cutoff).await {
                Ok(0) => break,
                Ok(count) => {
                    removed += count;
                    tokio::task::yield_now().await;
                }
                Err(error) => {
                    warn!(%error, "Не удалось очистить desktop OAuth; следующая попытка через минуту");
                    break;
                }
            }
        }
        if removed > 0 {
            info!(
                removed,
                "Удалены истёкшие desktop OAuth попытки, state и handoff"
            );
        }
    }
}
