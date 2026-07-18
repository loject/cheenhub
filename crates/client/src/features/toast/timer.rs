//! Управление временем жизни toast-уведомлений.

use dioxus::prelude::*;

use crate::features::application_focus::ApplicationFocusContext;
use crate::features::runtime::sleep_ms;

use super::provider::Toast;

pub(super) const TOAST_TTL_MS: u32 = 4_200;
const TOAST_TIMER_TICK_MS: u32 = 100;
const TOAST_EXIT_MS: u32 = 180;

pub(super) async fn run_toast_countdown(
    mut toasts: Signal<Vec<Toast>>,
    id: u64,
    application_focus: ApplicationFocusContext,
) {
    loop {
        sleep_ms(TOAST_TIMER_TICK_MS).await;

        let mut next_toasts = toasts();
        let Some(toast) = next_toasts.iter_mut().find(|toast| toast.id() == id) else {
            return;
        };
        if toast.exiting() {
            return;
        }
        let became_evictable = application_focus.is_focused() && toast.mark_focused_display();
        let application_focused = application_focus.is_focused();
        if toast.timer_paused(application_focused) {
            if became_evictable {
                toasts.set(next_toasts);
            }
            continue;
        }

        let expired = toast.tick(TOAST_TIMER_TICK_MS, application_focused);
        toasts.set(next_toasts);
        if expired {
            begin_dismiss_toast(&mut toasts, id);
            return;
        }
    }
}

pub(super) fn set_toast_hovered(toasts: &mut Signal<Vec<Toast>>, id: u64, hovered: bool) {
    let mut next_toasts = toasts();
    let Some(toast) = next_toasts.iter_mut().find(|toast| toast.id() == id) else {
        return;
    };
    if toast.set_hovered(hovered) {
        toasts.set(next_toasts);
    }
}

pub(super) fn begin_dismiss_toast(toasts: &mut Signal<Vec<Toast>>, id: u64) {
    let mut next_toasts = toasts();
    let Some(toast) = next_toasts.iter_mut().find(|toast| toast.id() == id) else {
        return;
    };
    if !toast.begin_exit() {
        return;
    }

    toasts.set(next_toasts);
    debug!(toast_id = id, "dismissing toast notification");

    let mut toasts = *toasts;
    spawn(async move {
        sleep_ms(TOAST_EXIT_MS).await;
        remove_toast(&mut toasts, id);
    });
}

fn remove_toast(toasts: &mut Signal<Vec<Toast>>, id: u64) {
    let mut next_toasts = toasts();
    let before = next_toasts.len();
    next_toasts.retain(|toast| toast.id() != id);
    if before == next_toasts.len() {
        return;
    }

    toasts.set(next_toasts);
    debug!(toast_id = id, "removed toast notification");
}

pub(super) fn advance_remaining(remaining_ms: &mut u32, elapsed_ms: u32, paused: bool) -> bool {
    if !paused {
        *remaining_ms = remaining_ms.saturating_sub(elapsed_ms);
    }
    *remaining_ms == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_does_not_advance_while_paused() {
        let mut remaining_ms = TOAST_TTL_MS;

        assert!(!advance_remaining(&mut remaining_ms, 1_000, true));
        assert_eq!(remaining_ms, TOAST_TTL_MS);
    }

    #[test]
    fn countdown_continues_from_remaining_time_after_pause() {
        let mut remaining_ms = 1_000;

        assert!(!advance_remaining(&mut remaining_ms, 400, false));
        assert!(!advance_remaining(&mut remaining_ms, 500, true));
        assert!(!advance_remaining(&mut remaining_ms, 500, false));
        assert!(advance_remaining(&mut remaining_ms, 100, false));
    }
}
