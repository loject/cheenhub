//! Заглушка DSP для сборок, отличных от Windows x86_64 MSVC.

/// Нейтральный процессор, сохраняющий входной PCM без изменений.
pub(super) struct Processor;

impl Processor {
    /// Создаёт нейтральный процессор без дополнительного состояния.
    pub(super) fn new() -> Self {
        Self
    }

    /// Оставляет входные сэмплы неизменными.
    pub(super) fn process(&mut self, _frame: &mut [f32]) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Показывает, что DSP недоступен на текущем target.
pub(super) fn available() -> bool {
    false
}
