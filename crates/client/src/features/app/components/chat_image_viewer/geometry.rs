//! Проверка viewport-rect и математика FLIP viewer.

use super::ChatImageViewerGeometry;

/// Равномерное преобразование shared-element относительно верхнего левого угла.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FlipTransform {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) scale: f64,
}

/// Проверяет пригодность rect для CSS transform.
pub(super) fn is_valid_geometry(rect: ChatImageViewerGeometry) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .all(|value| value.is_finite())
        && rect.width > 0.0
        && rect.height > 0.0
}

/// Строит contain-FLIP без неравномерного масштабирования.
pub(super) fn flip_transform(
    source: ChatImageViewerGeometry,
    target: ChatImageViewerGeometry,
) -> Option<FlipTransform> {
    if !is_valid_geometry(source) || !is_valid_geometry(target) {
        return None;
    }
    let scale = (source.width / target.width).min(source.height / target.height);
    scale.is_finite().then_some(FlipTransform {
        x: source.x + source.width / 2.0 - target.x - target.width / 2.0 * scale,
        y: source.y + source.height / 2.0 - target.y - target.height / 2.0 * scale,
        scale,
    })
}

#[cfg(test)]
pub(super) fn transformed_point(
    target: ChatImageViewerGeometry,
    transform: FlipTransform,
    local_x: f64,
    local_y: f64,
) -> (f64, f64) {
    (
        target.x + transform.x + local_x * transform.scale,
        target.y + transform.y + local_y * transform.scale,
    )
}
