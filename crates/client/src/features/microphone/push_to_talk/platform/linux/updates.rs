//! Публикация состояния portal в UI без доступа worker к Dioxus signals или storage.

use super::{Service, binding};
use crate::features::microphone::push_to_talk::{Availability, Status, Update};

impl Service {
    /// Сбрасывает подсказку только для захвата живого worker.
    pub(super) fn capture_started(&self) {
        self.updates.send_modify(|update| {
            if self.available.load(std::sync::atomic::Ordering::Acquire)
                && !self.stopped.load(std::sync::atomic::Ordering::Acquire)
            {
                update.restart_capture = false;
            }
        });
    }

    /// Меняет состояние и отзывает PCM epoch при отсутствии готовой сессии.
    pub(super) fn publish(&self, status: Status) {
        if status != Status::Ready {
            self.state.fail();
        }
        self.updates.send_modify(|update| update.status = status);
    }

    /// Публикует подтверждённое описание или удаление назначения для UI и storage.
    pub(super) fn binding_changed(&self, label: Option<&str>) {
        let current = binding::raw_label();
        if current == label.unwrap_or_default() && self.updates.borrow().binding.is_some() {
            return;
        }
        binding::set_label(label.unwrap_or_default());
        let key = if label.is_some() {
            binding::PushToTalkKey::assigned()
        } else {
            binding::PushToTalkKey::default()
        };
        self.updates
            .send_modify(|update| update.binding = Some(key));
    }

    /// Включает явное завершение ConfigureShortcuts в интерфейсе клиента.
    pub(super) fn configuring(&self, configuring: bool) {
        self.updates
            .send_modify(|update| update.configuring = configuring);
    }

    /// Хранит возможность назначения независимо от состояния текущей сессии.
    pub(super) fn available(&self, available: bool) {
        self.available
            .store(available, std::sync::atomic::Ordering::Release);
        self.updates.send_modify(|update| {
            update.availability = if available {
                Availability::Available
            } else {
                Availability::Unavailable
            }
        });
    }
}

/// Создаёт начальное состояние до фактического probe.
pub(super) fn initial() -> Update {
    Update {
        status: Status::Available,
        availability: Availability::Pending,
        binding: None,
        configuring: false,
        restart_capture: false,
    }
}
