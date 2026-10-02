//! Независимый сборщик истории активности голосового чата хоста.
//!
//! Сборщик не связан с системным metrics proxy: он опрашивает фичу `voice_chat`
//! и пишет снимки в базу настроек хоста. Время простоя backend остаётся пропуском
//! в истории, а не превращается в выдуманные нули.

use std::{sync::Arc, time::Duration};

use chrono::{Duration as ChronoDuration, Utc};
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

use crate::features::host_settings::domain::VoiceActivitySample;
use crate::features::voice_chat::application;
use crate::state::AppState;

/// Интервал между измерениями активности голосового чата.
pub(crate) const SAMPLE_INTERVAL: Duration = Duration::from_secs(10);
/// Глубина хранимой истории активности.
pub(crate) const HISTORY_WINDOW: ChronoDuration = ChronoDuration::hours(24);

/// Периодически сохраняет активность голосового чата в базу настроек хоста.
///
/// В снимок попадают оба показателя: голосовые подключения и видеоисточники.
pub(crate) struct HostActivityMonitor {
    state: AppState,
}

impl HostActivityMonitor {
    /// Создаёт сборщик для состояния приложения.
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Выполняет одно измерение: снимок активности, запись и очистку старых строк.
    ///
    /// Ошибка записи не прерывает цикл: она логируется, а следующий тик повторяет
    /// измерение. Возвращается, удалось ли сохранить снимок.
    pub(crate) async fn collect_once(&self) -> bool {
        let snapshot = application::activity_snapshot(&self.state).await;
        let now = Utc::now();
        let sample = VoiceActivitySample {
            id: Uuid::new_v4(),
            sampled_at: now,
            voice_connections: snapshot.voice_connections,
            video_sources: snapshot.video_sources,
        };

        if let Err(error) = self
            .state
            .host_settings_store
            .insert_voice_activity_sample(sample)
            .await
        {
            tracing::warn!(
                %error,
                voice_connections = snapshot.voice_connections,
                video_sources = snapshot.video_sources,
                "failed to store host voice activity sample; current counters stay available"
            );
            return false;
        }

        match self
            .state
            .host_settings_store
            .delete_voice_activity_samples_before(now - HISTORY_WINDOW)
            .await
        {
            Ok(0) => {}
            Ok(removed) => tracing::debug!(removed, "pruned host voice activity history"),
            Err(error) => tracing::warn!(
                %error,
                "failed to prune host voice activity history older than 24 hours"
            ),
        }

        true
    }

    /// Измеряет активность сразу после запуска и затем каждые 10 секунд.
    ///
    /// Пропущенные тики не накапливаются: при задержке цикл возвращается
    /// к следующему измерению без серии компенсирующих запросов.
    pub(crate) async fn run(self: Arc<Self>) {
        let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        tracing::info!(
            interval_seconds = SAMPLE_INTERVAL.as_secs(),
            history_hours = HISTORY_WINDOW.num_hours(),
            "starting host voice activity monitor"
        );

        loop {
            ticker.tick().await;
            self.collect_once().await;
        }
    }
}
