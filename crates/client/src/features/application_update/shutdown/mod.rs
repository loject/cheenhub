//! Платформенное закрытие основного окна после запуска обновления.

use std::rc::Rc;

mod native;

pub(crate) use native::use_application_update_shutdown;

/// Команда закрытия основного окна после запуска update-helper.
#[derive(Clone)]
pub(crate) struct ApplicationUpdateShutdown {
    close_after_update: Rc<dyn Fn()>,
}

impl ApplicationUpdateShutdown {
    /// Создает команду закрытия окна.
    fn new(close_after_update: impl Fn() + 'static) -> Self {
        Self {
            close_after_update: Rc::new(close_after_update),
        }
    }

    /// Закрывает основное окно после успешного запуска update-helper.
    pub(crate) fn close_after_update_started(&self) {
        (self.close_after_update)();
    }
}
