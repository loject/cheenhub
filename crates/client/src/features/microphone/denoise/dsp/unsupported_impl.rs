//! Нейтральный DSP для сборок без поддерживаемого RNNoise backend.

/// Нейтральный процессор, сохраняющий входной PCM без изменений.
pub(super) struct Processor;

impl Processor {
    /// Создаёт нейтральный процессор без дополнительного состояния.
    pub(super) fn new() -> Self {
        Self
    }

    /// Сохраняет PCM и возвращает `None`: оценка речи недоступна.
    pub(super) fn process(&mut self, _frame: &mut [f32]) -> Result<Option<f32>, &'static str> {
        Ok(None)
    }
}
