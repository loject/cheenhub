//! Общая логика изменения высоты встроенного чата в рабочей области.

use dioxus::prelude::*;

/// Базовая доля рабочей области, которую занимает встроенный чат.
pub(crate) const EMBEDDED_CHAT_DEFAULT_WORKSPACE_RATIO: f64 = 0.38;

const EMBEDDED_CHAT_MIN_WORKSPACE_RATIO: f64 = 0.24;
const VOICE_CHAT_MIN_WORKSPACE_RATIO: f64 = 0.16;

/// Ограничивает высоту встроенного чата так, чтобы голосовая область оставалась видимой.
pub(crate) fn clamp_embedded_chat_height(height_px: f64, workspace_height_px: f64) -> f64 {
    if workspace_height_px <= 0.0 {
        return height_px.max(0.0);
    }

    let min_text_height = workspace_height_px * EMBEDDED_CHAT_MIN_WORKSPACE_RATIO;
    let max_text_height = workspace_height_px * (1.0 - VOICE_CHAT_MIN_WORKSPACE_RATIO);
    height_px.clamp(min_text_height, max_text_height.max(min_text_height))
}

/// Завершает изменение высоты встроенного чата и пишет диагностический лог.
pub(crate) fn finish_embedded_chat_resize(
    mut resize_origin: Signal<Option<(f64, f64, f64)>>,
    height_px: Signal<Option<f64>>,
    workspace_id: &str,
) {
    let Some((_, _, workspace_height_px)) = resize_origin() else {
        return;
    };

    resize_origin.set(None);
    if let Some(height_px) = height_px() {
        let height_percent = if workspace_height_px > 0.0 {
            (height_px / workspace_height_px * 100.0).round() as i64
        } else {
            0
        };
        debug!(
            workspace_id,
            height_px = height_px.round() as i64,
            height_percent,
            "embedded text chat resize finished"
        );
    }
}
