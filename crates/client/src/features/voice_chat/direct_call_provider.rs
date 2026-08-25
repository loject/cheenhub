//! Провайдер signaling и глобального интерфейса личного звонка.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::{AudioPlaybackHandle, NotificationSound};
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};
use crate::features::runtime::sleep_ms;

use super::active_voice_notification_controls::ActiveVoiceNotificationControls;
use super::direct_call_notification_platform::{
    IncomingCallNotificationAction, clear_incoming_call_notification,
    show_incoming_call_notification, subscribe_incoming_call_notification_action_wakeups,
    take_pending_incoming_call_notification_action,
};
use super::direct_call_prompt::DirectCallPrompt;
use super::direct_call_realtime;
use super::direct_call_state::{DirectCallHandle, DirectCallUiState};
use super::state::VoiceConnectionHandle;

/// Предоставляет signaling личного звонка поверх готового голосового media-контекста.
#[component]
pub(super) fn DirectCallProvider(children: Element) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let realtime = use_context::<RealtimeHandle>();
    let voice = use_context::<VoiceConnectionHandle>();
    let playback = use_context::<AudioPlaybackHandle>();
    let state = use_signal(|| DirectCallUiState::Idle);
    let busy = use_signal(|| false);
    let direct_call = DirectCallHandle::new(state, busy, realtime.clone(), voice, current_user.id);
    let context = direct_call.clone();
    let mut displayed_incoming_call = use_signal(|| None);
    let mut prompt_exiting = use_signal(|| false);
    let prompt_generation = use_signal(|| 0_u64);
    use_context_provider(move || context.clone());

    let event_realtime = realtime.clone();
    let event_handle = direct_call.clone();
    use_hook(move || {
        spawn(async move {
            let mut events = direct_call_realtime::subscribe(&event_realtime);
            while let Some(event) = events.next().await {
                event_handle.apply_event(event);
            }
        })
    });

    let status_realtime = realtime.clone();
    let recovery_handle = direct_call.clone();
    use_hook(move || {
        spawn(async move {
            let mut statuses = status_realtime.subscribe_connection_status();
            while let Some(status) = statuses.next().await {
                if matches!(status, RealtimeConnectionStatus::Connected(_)) {
                    recovery_handle.recover();
                }
            }
        })
    });

    let notification_action_call = direct_call.clone();
    use_hook(move || {
        spawn(async move {
            let mut wakeups = subscribe_incoming_call_notification_action_wakeups();
            while wakeups.next().await.is_some() {
                apply_pending_incoming_call_notification_action(notification_action_call.clone())
                    .await;
            }
        })
    });

    let cold_start_action_call = direct_call.clone();
    use_effect(move || {
        if cold_start_action_call.incoming_call().is_none() {
            return;
        }
        let cold_start_action_call = cold_start_action_call.clone();
        spawn(async move {
            apply_pending_incoming_call_notification_action(cold_start_action_call).await;
        });
    });

    let incoming_sound_playback = playback.clone();
    let incoming_sound_call = direct_call.clone();
    let mut last_incoming_call_id = use_signal(|| None::<String>);
    use_effect(move || {
        let incoming_call = incoming_sound_call.incoming_call();
        let incoming_call_id = incoming_call.as_ref().map(|call| call.call_id.clone());
        if let Some(previous_call_id) = last_incoming_call_id.peek().as_ref()
            && incoming_call_id.as_ref() != Some(previous_call_id)
        {
            clear_incoming_call_notification(previous_call_id.clone());
        }
        if incoming_call_id.is_some() && incoming_call_id != *last_incoming_call_id.peek() {
            incoming_sound_playback.play_notification_sound(NotificationSound::MessageReceived);
            if let Some(call) = incoming_call {
                show_incoming_call_notification(
                    call.call_id,
                    call.conversation_id,
                    call.caller_nickname,
                );
            }
        }
        last_incoming_call_id.set(incoming_call_id);
    });

    let prompt_call = direct_call.clone();
    use_effect(move || {
        let incoming_call = prompt_call.incoming_call();
        let generation = prompt_generation.peek().wrapping_add(1);
        let mut prompt_generation = prompt_generation;
        prompt_generation.set(generation);

        if let Some(call) = incoming_call {
            displayed_incoming_call.set(Some(call));
            prompt_exiting.set(false);
            return;
        }
        if displayed_incoming_call.peek().is_none() || *prompt_exiting.peek() {
            return;
        }

        prompt_exiting.set(true);
        spawn(async move {
            sleep_ms(150).await;
            if *prompt_generation.peek() == generation {
                displayed_incoming_call.set(None);
                prompt_exiting.set(false);
            }
        });
    });

    rsx! {
        ActiveVoiceNotificationControls {}
        {children}
        if let Some(call) = displayed_incoming_call() {
            DirectCallPrompt {
                call,
                exiting: prompt_exiting(),
            }
        }
    }
}

async fn apply_pending_incoming_call_notification_action(direct_call: DirectCallHandle) {
    let Some(incoming_call) = direct_call.incoming_call() else {
        return;
    };
    let Some(action) = take_pending_incoming_call_notification_action().await else {
        return;
    };
    if action.call_id() != incoming_call.call_id {
        return;
    }

    match action {
        IncomingCallNotificationAction::Accept(_) => direct_call.accept(),
        IncomingCallNotificationAction::Decline(_) => direct_call.decline(),
    }
}
