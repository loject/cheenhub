//! Декодирование одной общей FIFO очереди сигналов: release не обгоняет предшествующий press.

use ashpd::{
    desktop::global_shortcuts::{Activated, Deactivated, ShortcutsChanged},
    zbus::Message,
};

/// Результат сигнала после проверки принадлежности текущей сессии и действию.
pub(super) enum Event {
    /// Изменение удержания выбранного сочетания.
    Held(bool),
    /// Новое описание сочетания; отсутствие означает удаление назначения.
    Changed(Option<String>),
    /// Сигнал другой сессии, shortcut или другого вида.
    Ignored,
}

/// Декодирует сигнал; вызывающая сторона читает все виды из одного упорядоченного stream.
///
/// # Errors
/// Некорректный payload закрывает сессию вместо сохранения прежнего удержания.
pub(super) fn decode(message: &Message, path: &str) -> Result<Event, ashpd::Error> {
    match message.header().member().map(|member| member.as_str()) {
        Some("Activated") => {
            let event: Activated = message.body().deserialize()?;
            Ok(
                if event.session_handle().as_str() == path
                    && event.shortcut_id() == super::SHORTCUT_ID
                {
                    Event::Held(true)
                } else {
                    Event::Ignored
                },
            )
        }
        Some("Deactivated") => {
            let event: Deactivated = message.body().deserialize()?;
            Ok(
                if event.session_handle().as_str() == path
                    && event.shortcut_id() == super::SHORTCUT_ID
                {
                    Event::Held(false)
                } else {
                    Event::Ignored
                },
            )
        }
        Some("ShortcutsChanged") => {
            let event: ShortcutsChanged = message.body().deserialize()?;
            Ok(if event.session_handle().as_str() == path {
                Event::Changed(super::shortcut_label(event.shortcuts()))
            } else {
                Event::Ignored
            })
        }
        _ => Ok(Event::Ignored),
    }
}

#[cfg(test)]
mod tests;

/// Отбрасывает старое удержание при смене epoch, сохраняя изменения и отзыв назначения.
///
/// # Errors
/// Отзыв или повреждённый сигнал требуют закрытия текущей сессии.
pub(super) fn fold_pending(
    message: &Message,
    path: &str,
    label: &mut Option<String>,
) -> Result<(), ashpd::Error> {
    match decode(message, path)? {
        Event::Changed(value) => {
            *label = value;
            if label.is_none() {
                return Err(ashpd::Error::NoResponse);
            }
        }
        Event::Held(_) | Event::Ignored => {}
    }
    Ok(())
}
