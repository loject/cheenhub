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
mod tests;
