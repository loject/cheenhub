//! Владелец окна выбора источника демонстрации экрана.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::source_picker::{
    ScreenShareSelection, ScreenShareSource, ScreenShareSourcePicker, ScreenShareSourcePickerState,
};
use super::source_preview::{self, MonitorPreview};

/// Загружает превью и обслуживает результат окна выбора.
#[component]
pub(super) fn ScreenSharePickerHost(mut open: Signal<bool>) -> Element {
    let toast = use_context::<ToastHandle>();
    let mut previews = use_resource(source_preview::load_previews);
    let picker_state = match previews.state().cloned() {
        UseResourceState::Pending => ScreenShareSourcePickerState::Loading,
        UseResourceState::Ready | UseResourceState::Stopped | UseResourceState::Paused => {
            match previews.read().clone() {
                None => ScreenShareSourcePickerState::Loading,
                Some(Ok(previews)) => ScreenShareSourcePickerState::Ready(
                    previews.into_iter().map(source_from_preview).collect(),
                ),
                Some(Err(error)) => ScreenShareSourcePickerState::Error(error.to_string()),
            }
        }
    };
    let confirmation_sources = match &picker_state {
        ScreenShareSourcePickerState::Ready(sources) => sources.clone(),
        ScreenShareSourcePickerState::Loading | ScreenShareSourcePickerState::Error(_) => {
            Vec::new()
        }
    };

    rsx! {
        ScreenShareSourcePicker {
            state: picker_state,
            on_close: move |_| {
                open.set(false);
                debug!("screen share source picker closed");
            },
            on_retry: move |_| {
                info!("retrying screen share source preview loading");
                previews.clear();
                previews.restart();
            },
            on_confirm: move |selection: ScreenShareSelection| {
                let summary = selection
                    .confirmation_summary(&confirmation_sources)
                    .unwrap_or_else(|| "Параметры демонстрации выбраны".to_owned());
                info!(
                    source_id = %selection.source_id,
                    resolution = ?selection.resolution,
                    frame_rate = ?selection.frame_rate,
                    "screen share source selection accepted for preview-only flow"
                );
                open.set(false);
                toast.info(format!("Выбрано: {summary}"));
            },
        }
    }
}

fn source_from_preview(preview: MonitorPreview) -> ScreenShareSource {
    ScreenShareSource {
        id: preview.id,
        name: preview.display_name,
        width: preview.width,
        height: preview.height,
        preview_url: format!(
            "data:image/png;base64,{}",
            STANDARD.encode(preview.png_bytes)
        ),
        is_primary: preview.primary,
    }
}

#[cfg(test)]
mod tests {
    use super::source_from_preview;
    use crate::features::screen_share::source_preview::MonitorPreview;

    #[test]
    fn converts_monitor_preview_to_picker_source_with_png_data_url() {
        let source = source_from_preview(MonitorPreview {
            id: "monitor-1".to_owned(),
            display_name: "Основной экран".to_owned(),
            width: 1_920,
            height: 1_080,
            primary: true,
            png_bytes: vec![137, 80, 78, 71],
        });

        assert_eq!(source.id, "monitor-1");
        assert_eq!(source.name, "Основной экран");
        assert_eq!(source.width, 1_920);
        assert_eq!(source.height, 1_080);
        assert!(source.is_primary);
        assert_eq!(source.preview_url, "data:image/png;base64,iVBORw==");
    }
}
