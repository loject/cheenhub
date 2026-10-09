//! PCM очередь с принадлежностью фрагментов конкретному удержанию Push-to-talk.

use super::super::push_to_talk::platform::Monitor;
use std::sync::{Arc, mpsc};

/// PCM фрагмент и поколение удержания в момент его захвата.
pub(super) struct Chunk {
    /// Mono samples, захваченные при открытом gate или в обычном режиме.
    pub(super) samples: Vec<f32>,
    /// Пара поколений удержания и назначения; смена любого отзывает фрагмент.
    ///
    /// `None` означает отсутствие разрешённого удержания; без monitor поле не используется.
    pub(super) epoch: Option<(u64, u64)>,
}

/// Владеет очередью и monitor на стороне кодировщика.
pub(super) struct Input {
    /// Ограниченная очередь PCM от платформенного захвата.
    pub(super) receiver: mpsc::Receiver<Chunk>,
    /// Общий с capture monitor; `None` для режимов без глобальной клавиши.
    pub(super) monitor: Option<Arc<Monitor>>,
}

/// Передаёт захваченный PCM и удерживает источник глобального ввода.
pub(super) struct Capture {
    /// Ограниченный sender; callback не ждёт освобождения очереди.
    pub(super) sender: mpsc::SyncSender<Chunk>,
    /// Источник текущего поколения удержания.
    pub(super) monitor: Option<Arc<Monitor>>,
}

/// Проверяет принадлежность PCM текущему удержанию до добавления в буфер.
///
/// Закрытый gate и смена поколения запрещают использование старого PCM.
pub(super) fn accepts_epoch(
    captured: Option<(u64, u64)>,
    current: Option<(u64, u64)>,
    ptt: bool,
) -> bool {
    !ptt || (captured.is_some() && captured == current)
}

/// Отделяет устройство-зависимый capture interval от момента нового нажатия.
pub(super) fn accepts_capture_epoch(
    previous: &mut Option<(u64, u64)>,
    current: Option<(u64, u64)>,
    ptt: bool,
) -> bool {
    let stable = *previous == current;
    *previous = current;
    !ptt || (current.is_some() && stable)
}

#[cfg(test)]
mod tests;
