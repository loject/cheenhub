//! Выбор действия для кнопки демонстрации экрана.

use super::ScreenShareStatus;
use super::backend::{ScreenShareCaptureSource, ScreenShareTargetQuality};
use super::source_picker::ScreenShareSelection;

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

/// Преобразует UI-модель выбора в платформенно-нейтральный источник захвата.
pub(super) fn capture_source_from_selection(
    selection: &ScreenShareSelection,
) -> ScreenShareCaptureSource {
    let (width, height) = selection.resolution.dimensions();
    ScreenShareCaptureSource::Selected {
        source_id: selection.source_id.clone(),
        target: ScreenShareTargetQuality {
            width,
            height,
            max_fps: selection.frame_rate.max_fps(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ScreenShareCaptureSource, ScreenShareTargetQuality, capture_source_from_selection,
    };
    use super::{ScreenShareControlIntent, screen_share_control_intent};
    use crate::features::screen_share::ScreenShareStatus;
    use crate::features::screen_share::source_picker::{
        ScreenShareFrameRate, ScreenShareResolution, ScreenShareSelection,
    };

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

    #[test]
    fn selected_picker_choice_becomes_platform_neutral_request_source() {
        let selection = ScreenShareSelection {
            source_id: "\\\\.\\DISPLAY2".to_owned(),
            resolution: ScreenShareResolution::P720,
            frame_rate: ScreenShareFrameRate::Fps15,
        };

        assert_eq!(
            capture_source_from_selection(&selection),
            ScreenShareCaptureSource::Selected {
                source_id: "\\\\.\\DISPLAY2".to_owned(),
                target: ScreenShareTargetQuality {
                    width: 1_280,
                    height: 720,
                    max_fps: 15,
                },
            }
        );
    }
}
