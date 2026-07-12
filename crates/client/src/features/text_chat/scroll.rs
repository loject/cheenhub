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
    if offset.y <= OLDER_PAGE_SCROLL_THRESHOLD
        && has_more()
        && !older_loading()
        && !initial_loading()
    {
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
    let bottom_gap = scroll_size.height - rect.size.height - offset.y;

    is_near_bottom.set(bottom_gap <= BOTTOM_SCROLL_THRESHOLD);
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
            let next_offset = offset_y + (scroll_size.height - height);
            let _ = element
                .scroll(
                    PixelsVector2D::new(0.0, next_offset.max(0.0)),
                    ScrollBehavior::Instant,
                )
                .await;
        }
    }
}
