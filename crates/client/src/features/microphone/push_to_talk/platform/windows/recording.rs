//! Запись одиночной привязки завершается отпусканием первой нажатой кнопки.

use super::super::super::key::PushToTalkKey;

#[derive(Default)]
/// Запоминает первую кнопку и игнорирует остальные до её отпускания.
pub(super) struct Recording {
    candidate: Option<PushToTalkKey>,
    initially_held: Vec<PushToTalkKey>,
}
impl Recording {
    /// Не перехватывает кнопки, удерживаемые до включения записи, до их отпускания.
    pub(super) fn with_initially_held(initially_held: Vec<PushToTalkKey>) -> Self {
        Self {
            candidate: None,
            initially_held,
        }
    }
    /// Сообщает, относится ли событие к записываемой кнопке или первому нажатию.
    pub(super) fn captures(&self, key: PushToTalkKey, pressed: bool) -> bool {
        !self.initially_held.contains(&key)
            && (self.candidate == Some(key) || (self.candidate.is_none() && pressed))
    }

    /// Принимает переход кнопки; первоначальное отпускание и autorepeat не завершают запись.
    pub(super) fn input(&mut self, key: PushToTalkKey, pressed: bool) -> Option<PushToTalkKey> {
        if self.initially_held.contains(&key) {
            if !pressed {
                self.initially_held.retain(|held| *held != key);
            }
            return None;
        }
        if pressed {
            self.candidate.get_or_insert(key);
            None
        } else if self.candidate == Some(key) {
            self.candidate.take()
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests;
