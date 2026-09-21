//! Выбор действия для кнопки демонстрации экрана.

use super::ScreenShareStatus;

/// Намерение, выбранное для нажатия кнопки демонстрации экрана.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScreenShareControlIntent {
    /// Открыть окно выбора источника и качества.
    OpenPicker,
    /// Начать захват без собственного окна выбора источника.
    Start,
    /// Остановить уже запущенный или запускаемый захват.
    Stop,
}

/// Выбирает действие с учётом возможностей платформы и текущего состояния.
pub(super) fn screen_share_control_intent(
    source_selection_available: bool,
    status: &ScreenShareStatus,
) -> ScreenShareControlIntent {
    if matches!(
        status,
        ScreenShareStatus::Live | ScreenShareStatus::Starting
    ) {
        ScreenShareControlIntent::Stop
    } else if source_selection_available {
        ScreenShareControlIntent::OpenPicker
    } else {
        ScreenShareControlIntent::Start
    }
}

#[cfg(test)]
mod tests {
    use super::{ScreenShareControlIntent, screen_share_control_intent};
    use crate::features::screen_share::ScreenShareStatus;

    #[test]
    fn opens_picker_when_platform_supports_source_selection() {
        assert_eq!(
            screen_share_control_intent(true, &ScreenShareStatus::Idle),
            ScreenShareControlIntent::OpenPicker
        );
    }

    #[test]
    fn starts_capture_when_source_selection_is_unavailable() {
        assert_eq!(
            screen_share_control_intent(false, &ScreenShareStatus::Idle),
            ScreenShareControlIntent::Start
        );
    }

    #[test]
    fn stops_active_capture_even_when_source_selection_is_available() {
        assert_eq!(
            screen_share_control_intent(true, &ScreenShareStatus::Live),
            ScreenShareControlIntent::Stop
        );
        assert_eq!(
            screen_share_control_intent(true, &ScreenShareStatus::Starting),
            ScreenShareControlIntent::Stop
        );
    }
}
