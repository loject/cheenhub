//! Измерение переполнения видимой области истории сообщений.

use std::rc::Rc;

use dioxus::prelude::*;

/// Создаёт локальный режим разделителей и команду повторного измерения истории.
pub(crate) fn use_history_overflow(
    list_element: Signal<Option<Rc<MountedData>>>,
) -> (Signal<bool>, Callback) {
    let mut overflowing = use_signal(|| false);
    let measure = use_callback(move |_| {
        let Some(element) = list_element.cloned() else {
            return;
        };
        spawn(async move {
            let scroll_size = match element.get_scroll_size().await {
                Ok(size) => size,
                Err(error) => {
                    debug!(?error, "chat history scroll size measurement failed");
                    return;
                }
            };
            let rect = match element.get_client_rect().await {
                Ok(rect) => rect,
                Err(error) => {
                    debug!(?error, "chat history viewport measurement failed");
                    return;
                }
            };
            if let Some(next) = measured_overflow(scroll_size.height, rect.size.height)
                && next != *overflowing.peek()
            {
                debug!(overflowing = next, "chat history date presentation changed");
                overflowing.set(next);
            }
        });
    });
    (overflowing, measure)
}

fn measured_overflow(scroll_height: f64, viewport_height: f64) -> Option<bool> {
    if !scroll_height.is_finite()
        || !viewport_height.is_finite()
        || scroll_height < 0.0
        || viewport_height <= 0.0
    {
        return None;
    }
    // Допуск исключает переключения из-за округления CSS pixels в scrollHeight.
    Some(scroll_height > viewport_height + 1.0)
}

#[cfg(test)]
mod tests;
