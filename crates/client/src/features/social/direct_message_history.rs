//! Жизненный цикл истории одного keyed-экземпляра личного диалога.
//!
//! Загружает историю при монтировании и сверяет её после social-событий,
//! переподключения realtime и возвращения фокуса приложению.

use cheenhub_contracts::realtime::SocialChangeReason;
use dioxus::prelude::*;
use futures_util::StreamExt;

use super::direct_message_state::DirectMessageState;
use super::presentation::{load_messages, refresh_messages};
use super::realtime::subscribe_social_events;
use crate::features::application_focus::ApplicationFocusContext;
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};

/// Подключает загрузку и сверку истории к жизненному циклу выбранного диалога.
pub(super) fn use_direct_message_history(
    realtime: RealtimeHandle,
    conversation_id: String,
    state: DirectMessageState,
    on_overview_changed: Callback<()>,
) {
    let application_focus = use_context::<ApplicationFocusContext>();
    let mut focus_initialized = use_signal(|| false);
    use_effect({
        let conversation_id = conversation_id.clone();
        let realtime = realtime.clone();
        move || {
            debug!(%conversation_id, "loading keyed direct message workspace");
            load_messages(
                realtime.clone(),
                conversation_id.clone(),
                state,
                on_overview_changed,
            );
        }
    });
    use_hook({
        let realtime = realtime.clone();
        let status_realtime = realtime.clone();
        let conversation_id = conversation_id.clone();
        move || {
            let ready_conversation_id = conversation_id.clone();
            spawn(async move {
                let mut statuses = status_realtime.subscribe_connection_status();
                let mut first_status = true;
                while let Some(status) = statuses.next().await {
                    if first_status {
                        first_status = false;
                        continue;
                    }
                    if matches!(status, RealtimeConnectionStatus::Connected(_)) {
                        refresh_messages(
                            status_realtime.clone(),
                            ready_conversation_id.clone(),
                            state,
                            on_overview_changed,
                        );
                    }
                }
            });
            spawn(async move {
                let mut events = subscribe_social_events(&realtime);
                while let Some(event) = events.next().await {
                    if event.reason == SocialChangeReason::DirectMessages
                        && event
                            .conversation_id
                            .as_deref()
                            .is_none_or(|changed_id| changed_id == conversation_id)
                    {
                        refresh_messages(
                            realtime.clone(),
                            conversation_id.clone(),
                            state,
                            on_overview_changed,
                        );
                    }
                }
            });
        }
    });
    use_effect(move || {
        let focused = application_focus.is_focused();
        if !*focus_initialized.peek() {
            focus_initialized.set(true);
            return;
        }
        if focused {
            refresh_messages(
                realtime.clone(),
                conversation_id.clone(),
                state,
                on_overview_changed,
            );
        }
    });
}
