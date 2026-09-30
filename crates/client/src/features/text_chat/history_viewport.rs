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
mod tests {
    use super::measured_overflow;

    #[test]
    fn date_bubbles_require_content_outside_the_viewport() {
        assert_eq!(measured_overflow(300.0, 600.0), Some(false));
        assert_eq!(measured_overflow(600.0, 600.0), Some(false));
        assert_eq!(measured_overflow(600.5, 600.0), Some(false));
        assert_eq!(measured_overflow(602.0, 600.0), Some(true));
    }

    #[test]
    fn resizing_back_to_fit_restores_date_lines() {
        assert_eq!(measured_overflow(900.0, 600.0), Some(true));
        assert_eq!(measured_overflow(900.0, 1000.0), Some(false));
    }

    #[test]
    fn hidden_or_invalid_viewports_do_not_replace_the_current_mode() {
        assert_eq!(measured_overflow(900.0, 0.0), None);
        assert_eq!(measured_overflow(900.0, -1.0), None);
        assert_eq!(measured_overflow(f64::NAN, 600.0), None);
        assert_eq!(measured_overflow(900.0, f64::INFINITY), None);
    }
}
