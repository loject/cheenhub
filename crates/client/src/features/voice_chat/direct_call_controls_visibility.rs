//! Локальное состояние видимости панели управления личным звонком.

use std::time::Duration;

use dioxus::prelude::*;
use web_time::Instant;

/// Задержка перед автоматическим скрытием панели управления.
const CONTROLS_IDLE_TIMEOUT: Duration = Duration::from_millis(3_200);

/// Управляет общей видимостью панели и её вложенных действий.
#[derive(Clone, Copy)]
pub(crate) struct DirectCallControlsVisibility {
    visible: Signal<bool>,
    locked: Signal<bool>,
    last_activity: Signal<Instant>,
}

impl DirectCallControlsVisibility {
    /// Создаёт состояние для одного экземпляра активного личного звонка.
    pub(crate) fn new(
        visible: Signal<bool>,
        locked: Signal<bool>,
        last_activity: Signal<Instant>,
    ) -> Self {
        Self {
            visible,
            locked,
            last_activity,
        }
    }

    /// Возвращает текущую видимость панели.
    pub(crate) fn is_visible(&self) -> bool {
        (self.visible)()
    }

    /// Показывает панель и начинает новый период ожидания.
    pub(crate) fn reveal(&self) {
        let was_visible = *self.visible.peek();
        let mut last_activity = self.last_activity;
        last_activity.set(Instant::now());
        if !was_visible {
            let mut visible = self.visible;
            visible.set(true);
            debug!("showing direct-call controls after user activity");
        }
    }

    /// Удерживает панель открытой, пока пользователь работает с popover.
    pub(crate) fn set_locked(&self, locked: bool) {
        let was_locked = *self.locked.peek();
        let mut lock_signal = self.locked;
        lock_signal.set(locked);
        if locked {
            let mut visible = self.visible;
            visible.set(true);
        } else {
            self.reveal();
        }
        if was_locked != locked {
            debug!(locked, "changed direct-call controls visibility lock");
        }
    }

    /// Скрывает панель, если период бездействия истёк и popover закрыт.
    pub(crate) fn hide_if_idle(&self) {
        if !*self.visible.peek()
            || *self.locked.peek()
            || self.last_activity.peek().elapsed() < CONTROLS_IDLE_TIMEOUT
        {
            return;
        }

        let mut visible = self.visible;
        visible.set(false);
        debug!("hiding idle direct-call controls");
    }
}
