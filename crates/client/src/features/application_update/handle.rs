//! Контекст управления пользовательским состоянием обновлений.

use std::time::Duration;

use dioxus::prelude::*;
use web_time::{Instant, SystemTime, UNIX_EPOCH};

use super::api::{self, UpdateCheckOutcome};
use super::download;
use super::storage;
use super::types::{AvailableUpdate, UpdateDownloadProgress, UpdateDownloadStatus};

const QUICK_DISMISS_SECONDS: u32 = 5 * 60;

/// Техническое состояние последней проверки обновлений.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UpdateCheckStatus {
    /// Проверка еще не выполнялась.
    Idle,
    /// Проверка выполняется прямо сейчас.
    Checking,
    /// Проверка завершилась, обновлений нет.
    Current {
        /// Время проверки в секундах UNIX epoch.
        checked_at_epoch_seconds: u64,
    },
    /// Проверка завершилась ошибкой.
    Failed {
        /// Сообщение об ошибке для пользователя.
        message: String,
    },
}

/// Пользовательское состояние обновлений для отрисовки интерфейса.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UpdateUiStatus {
    /// Проверка еще не выполнялась.
    Idle,
    /// Проверка выполняется прямо сейчас.
    Checking,
    /// Установлена актуальная версия.
    Current {
        /// Время проверки в секундах UNIX epoch.
        checked_at_epoch_seconds: u64,
    },
    /// Доступно обновление, которое можно показать пользователю.
    Available {
        /// Данные нового релиза.
        update: AvailableUpdate,
    },
    /// Пользователь отложил напоминание о найденном обновлении.
    Deferred {
        /// Данные нового релиза.
        update: AvailableUpdate,
        /// Время следующего напоминания в секундах UNIX epoch.
        until_epoch_seconds: u64,
    },
    /// Проверка завершилась ошибкой.
    Failed {
        /// Сообщение об ошибке для пользователя.
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ApplicationUpdateState {
    check_status: UpdateCheckStatus,
    available_update: Option<AvailableUpdate>,
    deferred_until_epoch_seconds: Option<u64>,
    notification_visible: bool,
    download_status: UpdateDownloadStatus,
}

impl Default for ApplicationUpdateState {
    fn default() -> Self {
        Self {
            check_status: UpdateCheckStatus::Idle,
            available_update: None,
            deferred_until_epoch_seconds: None,
            notification_visible: false,
            download_status: UpdateDownloadStatus::Idle,
        }
    }
}

/// Контекст обновлений клиентского приложения.
#[derive(Clone, Copy)]
pub(crate) struct ApplicationUpdateHandle {
    state: Signal<ApplicationUpdateState>,
}

impl ApplicationUpdateHandle {
    /// Создает контекст обновлений.
    pub(super) fn new(state: Signal<ApplicationUpdateState>) -> Self {
        Self { state }
    }

    /// Возвращает текущую версию клиентского приложения.
    pub(crate) fn current_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Возвращает пользовательское состояние обновлений.
    pub(crate) fn ui_status(&self) -> UpdateUiStatus {
        let state = (self.state)();
        if let Some(update) = state.available_update {
            if let Some(until_epoch_seconds) = state.deferred_until_epoch_seconds
                && until_epoch_seconds > now_epoch_seconds()
            {
                return UpdateUiStatus::Deferred {
                    update,
                    until_epoch_seconds,
                };
            }

            return UpdateUiStatus::Available { update };
        }

        match state.check_status {
            UpdateCheckStatus::Idle => UpdateUiStatus::Idle,
            UpdateCheckStatus::Checking => UpdateUiStatus::Checking,
            UpdateCheckStatus::Current {
                checked_at_epoch_seconds,
            } => UpdateUiStatus::Current {
                checked_at_epoch_seconds,
            },
            UpdateCheckStatus::Failed { message } => UpdateUiStatus::Failed { message },
        }
    }

    /// Проверяет, нужно ли показывать уведомление о доступном обновлении.
    pub(crate) fn should_show_notification(&self) -> Option<AvailableUpdate> {
        let state = (self.state)();
        if !state.notification_visible {
            return None;
        }

        let update = state.available_update?;
        if state
            .deferred_until_epoch_seconds
            .is_some_and(|until_epoch_seconds| until_epoch_seconds > now_epoch_seconds())
        {
            return None;
        }

        Some(update)
    }

    /// Возвращает состояние скачивания обновления.
    pub(crate) fn download_status(&self) -> UpdateDownloadStatus {
        (self.state)().download_status
    }

    /// Запускает проверку GitHub Releases.
    pub(crate) fn check_now(&self) {
        if matches!((self.state)().check_status, UpdateCheckStatus::Checking) {
            return;
        }

        let mut state = self.state;
        state.with_mut(|state| {
            state.check_status = UpdateCheckStatus::Checking;
        });

        info!("checking GitHub releases for application update");
        spawn(async move {
            match api::check_latest_release().await {
                Ok(UpdateCheckOutcome::Current) => {
                    info!("application update check completed without newer release");
                    state.with_mut(|state| {
                        state.check_status = UpdateCheckStatus::Current {
                            checked_at_epoch_seconds: now_epoch_seconds(),
                        };
                        state.available_update = None;
                        state.deferred_until_epoch_seconds = None;
                        state.notification_visible = false;
                        state.download_status = UpdateDownloadStatus::Idle;
                    });
                    storage::clear_deferral();
                }
                Ok(UpdateCheckOutcome::Available(update)) => {
                    let update_version = update.version.clone();
                    let stored_deferral = storage::load_deferral()
                        .filter(|deferral| deferral.version == update.version)
                        .filter(|deferral| deferral.until_epoch_seconds > now_epoch_seconds());
                    let deferred_until_epoch_seconds =
                        stored_deferral.map(|deferral| deferral.until_epoch_seconds);
                    let notification_visible = deferred_until_epoch_seconds.is_none();

                    info!(
                        update_version = %update.version,
                        update_tag = %update.tag,
                        deferred = deferred_until_epoch_seconds.is_some(),
                        "application update is available"
                    );
                    state.with_mut(|state| {
                        state.check_status = UpdateCheckStatus::Current {
                            checked_at_epoch_seconds: now_epoch_seconds(),
                        };
                        state.available_update = Some(update);
                        state.deferred_until_epoch_seconds = deferred_until_epoch_seconds;
                        state.notification_visible = notification_visible;
                        let keep_download_status = matches!(
                            &state.download_status,
                            UpdateDownloadStatus::Downloading { .. }
                        ) || matches!(
                            &state.download_status,
                            UpdateDownloadStatus::Downloaded { version, .. }
                                if version == &update_version
                        );
                        if !keep_download_status {
                            state.download_status = UpdateDownloadStatus::Idle;
                        }
                    });
                }
                Err(message) => {
                    warn!(%message, "application update check failed");
                    state.with_mut(|state| {
                        state.check_status = UpdateCheckStatus::Failed { message };
                        state.notification_visible = false;
                    });
                }
            }
        });
    }

    /// Откладывает напоминание о найденном обновлении.
    pub(crate) fn defer_update(&self, delay: UpdateDeferralDelay) {
        self.defer_update_seconds(delay.seconds());
    }

    /// Скачивает установщик доступного обновления для текущей платформы.
    pub(crate) fn download_update(&self) {
        let Some(update) = (self.state)().available_update else {
            warn!("application update download requested without available update");
            return;
        };

        let version = update.version.clone();
        if matches!(
            (self.state)().download_status,
            UpdateDownloadStatus::Downloading { .. }
        ) {
            debug!(update_version = %version, "application update download is already running");
            return;
        }
        if matches!(
            (self.state)().download_status,
            UpdateDownloadStatus::Downloaded {
                version: ref downloaded_version,
                ..
            } if downloaded_version == &version
        ) {
            debug!(update_version = %version, "application update was already downloaded");
            return;
        }

        let Some(asset) = update.download_asset else {
            let message = "Для этой платформы в релизе нет подходящего установщика.".to_owned();
            let mut state = self.state;
            state.with_mut(|state| {
                state.download_status = UpdateDownloadStatus::Failed {
                    version: version.clone(),
                    message: message.clone(),
                };
            });
            warn!(update_version = %version, "no application update asset for current platform");
            return;
        };

        let mut state = self.state;
        let initial_progress = UpdateDownloadProgress {
            downloaded_bytes: 0,
            total_bytes: (asset.size_bytes > 0).then_some(asset.size_bytes),
            bytes_per_second: 0,
        };
        state.with_mut(|state| {
            state.download_status = UpdateDownloadStatus::Downloading {
                version: version.clone(),
                progress: initial_progress,
            };
        });

        spawn(async move {
            let started_at = Instant::now();
            let mut progress_state = state;
            let progress_version = version.clone();
            match download::download_update_asset(asset, move |mut progress| {
                let elapsed_seconds = started_at.elapsed().as_secs_f64();
                if elapsed_seconds > 0.0 {
                    progress.bytes_per_second =
                        (progress.downloaded_bytes as f64 / elapsed_seconds) as u64;
                }
                progress_state.with_mut(|state| {
                    state.download_status = UpdateDownloadStatus::Downloading {
                        version: progress_version.clone(),
                        progress,
                    };
                });
            })
            .await
            {
                Ok(file) => {
                    info!(
                        update_version = %version,
                        saved_path = %file.path,
                        "application update download completed"
                    );
                    state.with_mut(|state| {
                        state.download_status = UpdateDownloadStatus::Downloaded { version, file };
                    });
                }
                Err(message) => {
                    warn!(
                        update_version = %version,
                        %message,
                        "application update download failed"
                    );
                    state.with_mut(|state| {
                        state.download_status = UpdateDownloadStatus::Failed { version, message };
                    });
                }
            }
        });
    }

    /// Запускает установщик уже скачанного обновления.
    pub(crate) fn install_downloaded_update(&self) -> bool {
        let UpdateDownloadStatus::Downloaded { version, file } = (self.state)().download_status
        else {
            warn!("application update install requested without downloaded update");
            return false;
        };

        match download::install_downloaded_update(&version, &file) {
            Ok(()) => {
                info!(
                    update_version = %version,
                    update_path = %file.path,
                    "application update installer started"
                );
                true
            }
            Err(message) => {
                warn!(
                    update_version = %version,
                    %message,
                    "application update installer failed to start"
                );
                let mut state = self.state;
                state.with_mut(|state| {
                    state.download_status = UpdateDownloadStatus::Failed { version, message };
                });
                false
            }
        }
    }

    /// Скрывает уведомление об обновлении на пять минут.
    pub(crate) fn dismiss_update_for_five_minutes(&self) {
        self.defer_update_seconds(QUICK_DISMISS_SECONDS);
    }

    fn defer_update_seconds(&self, delay_seconds: u32) {
        let Some(update) = (self.state)().available_update else {
            return;
        };
        let until_epoch_seconds = now_epoch_seconds().saturating_add(u64::from(delay_seconds));
        storage::save_deferral(&update.version, until_epoch_seconds);

        let mut state = self.state;
        state.with_mut(|state| {
            state.deferred_until_epoch_seconds = Some(until_epoch_seconds);
            state.notification_visible = false;
        });

        info!(
            update_version = %update.version,
            delay_seconds,
            until_epoch_seconds,
            "deferred application update notification"
        );
    }

    /// Сбрасывает отсрочку и снова показывает уведомление об обновлении.
    pub(crate) fn show_deferred_update_now(&self) {
        storage::clear_deferral();
        let mut state = self.state;
        state.with_mut(|state| {
            state.deferred_until_epoch_seconds = None;
            if state.available_update.is_some() {
                state.notification_visible = true;
            }
        });
        info!("cleared application update deferral");
    }

    /// Показывает уведомление после истечения сохраненной отсрочки.
    pub(crate) fn release_deferral_if_due(&self, version: &str, until_epoch_seconds: u64) {
        if until_epoch_seconds > now_epoch_seconds() {
            return;
        }

        let mut state = self.state;
        state.with_mut(|state| {
            let same_version = state
                .available_update
                .as_ref()
                .is_some_and(|update| update.version == version);
            if same_version && state.deferred_until_epoch_seconds == Some(until_epoch_seconds) {
                state.deferred_until_epoch_seconds = None;
                state.notification_visible = true;
                storage::clear_deferral();
                info!(
                    update_version = %version,
                    "application update deferral expired"
                );
            }
        });
    }
}

/// Доступные интервалы отсрочки напоминания об обновлении.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UpdateDeferralDelay {
    /// Напомнить через один час.
    OneHour,
    /// Напомнить через четыре часа.
    FourHours,
    /// Напомнить завтра.
    Tomorrow,
    /// Напомнить через неделю.
    OneWeek,
}

impl UpdateDeferralDelay {
    /// Возвращает все интервалы отсрочки.
    pub(crate) const fn all() -> [Self; 4] {
        [
            Self::OneHour,
            Self::FourHours,
            Self::Tomorrow,
            Self::OneWeek,
        ]
    }

    /// Возвращает значение для HTML select.
    pub(crate) const fn value(self) -> &'static str {
        match self {
            Self::OneHour => "one_hour",
            Self::FourHours => "four_hours",
            Self::Tomorrow => "tomorrow",
            Self::OneWeek => "one_week",
        }
    }

    /// Возвращает подпись интервала.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::OneHour => "Через час",
            Self::FourHours => "Через 4 часа",
            Self::Tomorrow => "Завтра",
            Self::OneWeek => "Через неделю",
        }
    }

    /// Возвращает длительность отсрочки в секундах.
    pub(crate) const fn seconds(self) -> u32 {
        match self {
            Self::OneHour => 60 * 60,
            Self::FourHours => 4 * 60 * 60,
            Self::Tomorrow => 24 * 60 * 60,
            Self::OneWeek => 7 * 24 * 60 * 60,
        }
    }

    /// Разбирает значение из HTML select.
    pub(crate) fn from_value(value: &str) -> Self {
        Self::all()
            .into_iter()
            .find(|delay| delay.value() == value)
            .unwrap_or(Self::Tomorrow)
    }
}

/// Возвращает текущее время в секундах UNIX epoch.
pub(crate) fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}
