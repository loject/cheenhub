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
mod tests {
    use super::*;

    fn anchor() -> ScrollAnchor {
        ScrollAnchor {
            message_id: "long".into(),
            inside_y: 740.0,
            preceding_ids: vec!["previous".into(), "older".into()],
            revision: 2,
        }
    }

    #[test]
    fn reading_inside_a_long_message_keeps_its_internal_offset() {
        let rows = vec![
            ("above".into(), -1100.0, 100.0),
            ("long".into(), -640.0, 1800.0),
            ("next".into(), 1200.0, 80.0),
        ];
        assert_eq!(
            choose_anchor(&rows, 100.0, 600.0),
            Some(("long".into(), 740.0))
        );
        assert_eq!(
            restored_offset(2000.0, -300.0, 100.0, 1800.0, 740.0),
            2340.0
        );
    }

    #[test]
    fn removed_anchor_uses_the_nearest_surviving_predecessor() {
        assert_eq!(
            anchor().resolve(&["older".into(), "previous".into(), "next".into()]),
            Some(("previous".into(), 0.0))
        );
        assert_eq!(
            anchor().resolve(&["older".into(), "next".into()]),
            Some(("older".into(), 0.0))
        );
    }

    #[test]
    fn existing_anchor_keeps_offset_even_after_new_messages_are_inserted() {
        assert_eq!(
            anchor().resolve(&["new".into(), "long".into()]),
            Some(("long".into(), 740.0))
        );
    }

    #[test]
    fn shortened_message_clamps_the_internal_offset() {
        assert_eq!(restored_offset(500.0, 60.0, 100.0, 120.0, 740.0), 579.0);
    }

    #[test]
    fn no_surviving_predecessor_uses_the_first_remaining_message() {
        assert_eq!(
            anchor().resolve(&["next".into()]),
            Some(("next".into(), 0.0))
        );
        assert_eq!(anchor().resolve(&[]), None);
    }

    #[test]
    fn gap_above_the_first_visible_message_does_not_create_a_negative_offset() {
        assert_eq!(
            choose_anchor(&[("below".into(), 140.0, 80.0)], 100.0, 600.0),
            Some(("below".into(), 0.0))
        );
        assert_eq!(
            choose_anchor(&[("outside".into(), 700.0, 80.0)], 100.0, 600.0),
            None
        );
    }
}
