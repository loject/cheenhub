//! Выбор сообщения и внутреннего смещения для восстановления позиции чтения.

/// Позиция чтения относительно сообщения и его предшественников.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScrollAnchor {
    /// Сообщение, пересекающее верхнюю границу окна.
    pub(super) message_id: String,
    /// Смещение верхней границы окна внутри сообщения.
    pub(super) inside_y: f64,
    /// Предыдущие сообщения от ближайшего к самому раннему.
    pub(super) preceding_ids: Vec<String>,
    /// Версия прокрутки при измерении позиции.
    pub(super) revision: u64,
}

/// Выбирает первое сообщение, видимое в окне, включая очень длинное.
pub(super) fn choose_anchor(
    rows: &[(String, f64, f64)],
    viewport_top: f64,
    viewport_bottom: f64,
) -> Option<(String, f64)> {
    rows.iter()
        .find(|(_, top, height)| {
            top.is_finite()
                && height.is_finite()
                && *height > 0.0
                && top + height > viewport_top
                && *top < viewport_bottom
        })
        .map(|(id, top, _)| (id.clone(), (viewport_top - top).max(0.0)))
}

impl ScrollAnchor {
    /// Находит исходное сообщение либо ближайшее сохранившееся сообщение выше.
    pub(super) fn resolve(&self, ids: &[String]) -> Option<(String, f64)> {
        if ids.contains(&self.message_id) {
            return Some((self.message_id.clone(), self.inside_y));
        }
        self.preceding_ids
            .iter()
            .find(|id| ids.contains(id))
            .or_else(|| ids.first())
            .map(|id| (id.clone(), 0.0))
    }
}

/// Рассчитывает прокрутку с ограничением смещения текущей высотой сообщения.
pub(super) fn restored_offset(
    scroll_y: f64,
    row_top: f64,
    viewport_top: f64,
    height: f64,
    inside_y: f64,
) -> f64 {
    (scroll_y + row_top - viewport_top + inside_y.min((height - 1.0).max(0.0))).max(0.0)
}

#[cfg(test)]
mod tests;
