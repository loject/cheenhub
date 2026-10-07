//! Управление жизненным циклом процесса: приём сигналов остановки и отказ от новых подключений.
//!
//! Модуль не относится к продуктовым фичам: он связывает сигналы операционной
//! системы с HTTP-оболочкой, realtime-службой и health-маршрутом. Единственное
//! состояние — фаза процесса, поэтому оно общее для всего процесса, а не часть
//! `AppState`.

use std::{sync::OnceLock, time::Duration};

use anyhow::Context;

#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::watch;
use tracing::{info, warn};

/// Фаза жизненного цикла процесса.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LifecyclePhase {
    /// Процесс принимает новые подключения и обслуживает трафик.
    Serving,
    /// Процесс завершает активные соединения и новых не принимает.
    Draining,
}

/// Фаза процесса, общая для HTTP-оболочки, realtime и health-маршрута.
///
/// Экземпляр создаётся один раз на процесс: все подсистемы читают одну и ту же
/// фазу, поэтому состояние хранится в общем владельце, а не в `AppState`.
pub(crate) struct Lifecycle {
    phase: watch::Sender<LifecyclePhase>,
    active_realtime: watch::Sender<usize>,
}

impl Lifecycle {
    /// Создаёт процесс в фазе обслуживания трафика.
    pub(crate) fn new() -> Self {
        let (phase, _) = watch::channel(LifecyclePhase::Serving);
        let (active_realtime, _) = watch::channel(0);
        Self {
            phase,
            active_realtime,
        }
    }

    /// Удерживает runtime живым до завершения обработчика realtime-сессии.
    ///
    /// После начала остановки новые обработчики не регистрируются. Владение
    /// guard заканчивается после закрытия транспорта и очистки его ресурсов.
    pub(crate) fn track_realtime_session(&self) -> Option<RealtimeSessionGuard<'_>> {
        if self.is_draining() {
            return None;
        }
        self.active_realtime.send_modify(|active| *active += 1);
        let guard = RealtimeSessionGuard { lifecycle: self };
        if self.is_draining() {
            return None;
        }
        Some(guard)
    }

    /// Ожидает закрытия всех зарегистрированных realtime-обработчиков.
    ///
    /// Вызывается после остановки слушателей, когда новые сессии уже запрещены.
    pub(crate) async fn wait_for_realtime_sessions(&self) {
        let mut active = self.active_realtime.subscribe();
        loop {
            if *active.borrow_and_update() == 0 {
                return;
            }
            if active.changed().await.is_err() {
                return;
            }
        }
    }

    /// Возвращает текущую фазу процесса.
    pub(crate) fn phase(&self) -> LifecyclePhase {
        *self.phase.borrow()
    }

    /// Сообщает, завершает ли процесс активные соединения.
    pub(crate) fn is_draining(&self) -> bool {
        self.phase() == LifecyclePhase::Draining
    }

    /// Подписывается на смену фазы процесса.
    ///
    /// Текущая фаза приходит сразу, поэтому подписчик не обязан отдельно
    /// запрашивать её перед ожиданием.
    pub(crate) fn subscribe(&self) -> watch::Receiver<LifecyclePhase> {
        self.phase.subscribe()
    }

    /// Переводит процесс в фазу завершения и запрещает новые подключения.
    ///
    /// Повторный вызов не меняет уже начатое завершение, поэтому обработчики
    /// могут вызывать его независимо друг от друга.
    pub(crate) fn begin_draining(&self) {
        if self.is_draining() {
            return;
        }
        info!("backend entered draining phase and no longer accepts new connections");
        // send_replace обновляет значение всегда, пока есть сам отправитель.
        // Обычный send вернул бы ошибку и ничего не изменил бы, когда на фазу
        // ещё никто не подписан, и тогда завершение не наступило бы никогда.
        self.phase.send_replace(LifecyclePhase::Draining);
    }

    /// Ожидает перехода процесса в фазу завершения.
    ///
    /// Возвращает управление сразу, если завершение уже начато.
    pub(crate) async fn wait_for_draining(&self) {
        let mut phase = self.subscribe();
        if *phase.borrow_and_update() == LifecyclePhase::Draining {
            return;
        }
        if phase.changed().await.is_err() {
            warn!("lifecycle phase sender dropped while waiting for draining");
        }
    }
}

/// Учёт времени жизни одного обработчика realtime-транспорта.
///
/// Освобождение guard сообщает процессу, что очистка ресурсов завершена.
pub(crate) struct RealtimeSessionGuard<'a> {
    lifecycle: &'a Lifecycle,
}

impl Drop for RealtimeSessionGuard<'_> {
    fn drop(&mut self) {
        self.lifecycle
            .active_realtime
            .send_modify(|active| *active -= 1);
    }
}

/// Владелец фазы процесса, общий для всех подсистем.
static LIFECYCLE: OnceLock<Lifecycle> = OnceLock::new();

/// Возвращает общий владелец фазы процесса, создавая его при первом обращении.
pub(crate) fn lifecycle() -> &'static Lifecycle {
    LIFECYCLE.get_or_init(Lifecycle::new)
}

/// Ожидает сигнал остановки процесса от операционной системы.
///
/// На Unix Docker и systemd отправляют `SIGTERM`, а `SIGINT` используется для
/// ручной остановки во время разработки. На остальных платформах ожидание
/// использует доступный Tokio сигнал Ctrl+C.
pub(crate) async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        let terminate = match signal(SignalKind::terminate()) {
            Ok(terminate) => terminate,
            Err(error) => {
                warn!(
                    %error,
                    "failed to listen for terminate signal; falling back to interrupt signal"
                );
                if let Err(error) = tokio::signal::ctrl_c().await {
                    warn!(%error, "failed to listen for interrupt signal");
                }
                return;
            }
        };

        let mut terminate = terminate;
        tokio::select! {
            result = tokio::signal::ctrl_c() => match result {
                Ok(()) => info!("received interrupt signal; shutting down"),
                Err(error) => warn!(%error, "failed to listen for interrupt signal"),
            },
            signal = terminate.recv() => match signal {
                Some(()) => info!("received terminate signal; shutting down"),
                None => warn!("terminate signal stream ended; shutting down"),
            },
        }
    }

    #[cfg(not(unix))]
    match tokio::signal::ctrl_c().await {
        Ok(()) => info!("received interrupt signal; shutting down"),
        Err(error) => warn!(%error, "failed to listen for interrupt signal"),
    }
}

/// Ожидает завершение realtime-службы перед уничтожением runtime.
///
/// Получает задачу слушателя, чтобы завершение HTTP не отменяло её работу.
/// Ожидание слушателя и обработчиков ограничено 25 секундами; по таймауту
/// процесс сообщает об ошибке и завершает оставшиеся фоновые задачи.
pub(crate) async fn finish_realtime(
    mut task: tokio::task::JoinHandle<anyhow::Result<()>>,
) -> anyhow::Result<()> {
    let completion = async {
        (&mut task)
            .await
            .context("realtime listener task failed during shutdown")??;
        lifecycle().wait_for_realtime_sessions().await;
        Ok(())
    };
    match tokio::time::timeout(Duration::from_secs(25), completion).await {
        Ok(result) => {
            info!("realtime shutdown completed before runtime exit");
            result
        }
        Err(error) => {
            task.abort();
            warn!(
                timeout_seconds = 25,
                "realtime shutdown timed out; cancelling remaining tasks"
            );
            Err(error.into())
        }
    }
}

#[cfg(test)]
mod tests;
