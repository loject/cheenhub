//! Gate удержания клавиши, независимый от Windows API.

/// Закрывает передачу при потере ввода и требует подтверждённого состояния после восстановления.
#[derive(Debug, Clone, Default)]
pub(in crate::features::microphone) struct Gate {
    held: bool,
    ready: bool,
}

impl Gate {
    /// Принимает событие клавиши; без готового monitor gate остаётся закрытым.
    pub(in crate::features::microphone) fn event(&mut self, held: bool) {
        self.held = held;
    }
    /// Восстанавливает gate по подтверждённому состоянию ввода.
    pub(in crate::features::microphone) fn recover(&mut self, held: bool) {
        self.held = held;
        self.ready = true;
    }
    /// Немедленно закрывает передачу до восстановления monitor.
    pub(in crate::features::microphone) fn failed(&mut self) {
        self.ready = false;
    }
    /// Возвращает удержание только при исправном источнике событий.
    pub(in crate::features::microphone) fn active(&self) -> bool {
        self.ready && self.held
    }
}

#[cfg(test)]
mod tests;
