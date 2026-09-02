//! Поведение прокрутки текстового чата.

use std::rc::Rc;

use dioxus::prelude::dioxus_elements::geometry::PixelsVector2D;
use dioxus::prelude::*;

const BOTTOM_SCROLL_THRESHOLD: f64 = 24.0;
const OLDER_PAGE_SCROLL_THRESHOLD: f64 = 48.0;

#[derive(Clone, Copy)]
pub(crate) enum ScrollCommand {
    /// Мгновенно прокручивает список к последнему сообщению.
    Bottom,
    /// Плавно прокручивает список к последнему сообщению.
    SmoothBottom,
    /// Сохраняет видимую позицию после добавления сообщений выше текущего окна.
    Preserve { offset_y: f64, height: f64 },
}

pub(super) async fn update_scroll_state(
    element: Rc<MountedData>,
    is_near_bottom: Signal<bool>,
    has_more: Signal<bool>,
    older_loading: Signal<bool>,
    initial_loading: Signal<bool>,
    load_older: Callback,
) {
    update_near_bottom_state(element.clone(), is_near_bottom).await;
    let Ok(offset) = element.get_scroll_offset().await else {
        return;
    };
    if should_load_older(offset.y, has_more(), older_loading(), initial_loading()) {
        load_older.call(());
    }
}

pub(crate) async fn capture_scroll_position(element: Rc<MountedData>) -> Option<(f64, f64)> {
    let offset = element.get_scroll_offset().await.ok()?;
    let scroll_size = element.get_scroll_size().await.ok()?;

    Some((offset.y, scroll_size.height))
}

/// Обновляет признак того, что пользователь находится рядом с нижней границей списка.
pub(crate) async fn update_near_bottom_state(
    element: Rc<MountedData>,
    mut is_near_bottom: Signal<bool>,
) {
    let Ok(offset) = element.get_scroll_offset().await else {
        return;
    };
    let Ok(scroll_size) = element.get_scroll_size().await else {
        return;
    };
    let Ok(rect) = element.get_client_rect().await else {
        return;
    };
    is_near_bottom.set(is_offset_near_bottom(
        offset.y,
        scroll_size.height,
        rect.size.height,
    ));
}

/// Применяет отложенную команду прокрутки к списку сообщений.
pub(crate) async fn apply_scroll_command(element: Rc<MountedData>, command: ScrollCommand) {
    match command {
        ScrollCommand::Bottom => {
            let Ok(scroll_size) = element.get_scroll_size().await else {
                return;
            };
            let _ = element
                .scroll(
                    PixelsVector2D::new(0.0, scroll_size.height),
                    ScrollBehavior::Instant,
                )
                .await;
        }
        ScrollCommand::SmoothBottom => {
            let Ok(scroll_size) = element.get_scroll_size().await else {
                return;
            };
            let _ = element
                .scroll(
                    PixelsVector2D::new(0.0, scroll_size.height),
                    ScrollBehavior::Smooth,
                )
                .await;
        }
        ScrollCommand::Preserve { offset_y, height } => {
            let Ok(scroll_size) = element.get_scroll_size().await else {
                return;
            };
            let next_offset = preserved_scroll_offset(offset_y, height, scroll_size.height);
            let _ = element
                .scroll(
                    PixelsVector2D::new(0.0, next_offset),
                    ScrollBehavior::Instant,
                )
                .await;
        }
    }
}

fn should_load_older(
    offset_y: f64,
    has_more: bool,
    older_loading: bool,
    initial_loading: bool,
) -> bool {
    offset_y <= OLDER_PAGE_SCROLL_THRESHOLD && has_more && !older_loading && !initial_loading
}

fn is_offset_near_bottom(offset_y: f64, scroll_height: f64, viewport_height: f64) -> bool {
    scroll_height - viewport_height - offset_y <= BOTTOM_SCROLL_THRESHOLD
}

fn preserved_scroll_offset(offset_y: f64, previous_height: f64, next_height: f64) -> f64 {
    (offset_y + next_height - previous_height).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_history_loads_only_at_the_top_in_an_idle_ready_list() {
        assert!(should_load_older(
            OLDER_PAGE_SCROLL_THRESHOLD,
            true,
            false,
            false
        ));
        assert!(!should_load_older(
            OLDER_PAGE_SCROLL_THRESHOLD + 0.1,
            true,
            false,
            false
        ));
        assert!(!should_load_older(0.0, false, false, false));
        assert!(!should_load_older(0.0, true, true, false));
        assert!(!should_load_older(0.0, true, false, true));
    }

    #[test]
    fn bottom_detection_includes_the_existing_threshold() {
        assert!(is_offset_near_bottom(476.0, 1_000.0, 500.0));
        assert!(!is_offset_near_bottom(475.9, 1_000.0, 500.0));
        assert!(is_offset_near_bottom(0.0, 300.0, 500.0));
    }

    #[test]
    fn preserving_scroll_adds_content_growth_above_the_viewport() {
        assert_eq!(preserved_scroll_offset(20.0, 1_000.0, 1_450.0), 470.0);
    }

    #[test]
    fn preserving_scroll_never_returns_a_negative_offset() {
        assert_eq!(preserved_scroll_offset(20.0, 1_000.0, 500.0), 0.0);
    }
}
