//! Провайдер signaling и глобального интерфейса личного звонка.

use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::AudioPlaybackHandle;
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};
use crate::features::toast::ToastHandle;

use super::active_voice_notification_controls::ActiveVoiceNotificationControls;
use super::direct_call_notification_platform::{
    IncomingCallNotificationAction, clear_incoming_call_notification,
    show_incoming_call_notification, subscribe_incoming_call_notification_action_wakeups,
    take_pending_incoming_call_notification_action,
};
use super::direct_call_realtime;
use super::direct_call_state::{DirectCallHandle, DirectCallUiState};
use super::direct_call_toast::{DirectCallToast, DirectCallToastAction};
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
    let owned_call_id = use_signal(|| None);
    let direct_call = DirectCallHandle::new(
        state,
        busy,
        owned_call_id,
        realtime.clone(),
        voice,
        current_user.id,
    );
    let context = direct_call.clone();
    let toasts = use_context::<ToastHandle>();
    let mut incoming_toast_state = use_signal(IncomingCallToastState::default);
    use_context_provider(move || context.clone());

    // Обрабатываем действия toast в области владения сигналами DirectCallProvider.
    // ToastProvider получает только канал, без ссылок на дочерние Dioxus Signals.
    let action_call = direct_call.clone();
    let mut action_state = incoming_toast_state;
    let action_toasts = toasts;
    let incoming_toast_action_sender = use_hook(move || {
        let (sender, mut receiver) = mpsc::unbounded::<(String, DirectCallToastAction)>();
        spawn(async move {
            while let Some((call_id, action)) = receiver.next().await {
                if !action_call
                    .incoming_call()
                    .is_some_and(|incoming| incoming.call_id == call_id)
                {
                    continue;
                }

                match action {
                    DirectCallToastAction::Accept => {
                        debug!(call_id = %call_id, "accepting incoming direct call from toast");
                        action_call.accept();
                    }
                    DirectCallToastAction::Decline => {
                        debug!(call_id = %call_id, "declining incoming direct call from toast");
                        action_call.decline();
                    }
                    DirectCallToastAction::Ignore => {
                        let mut state = action_state.peek().cloned();
                        if state.ignore(&call_id) {
                            action_state.set(state);
                            action_toasts.dismiss_custom(&incoming_toast_key(&call_id));
                            clear_incoming_call_notification(call_id.clone());
                            info!(call_id = %call_id, "ignored incoming direct call toast and notification locally");
                        }
                    }
                }
            }
        });
        sender
    });

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

    // Android передаёт видимость Activity через lifecycle; на остальных платформах она постоянна.
    let mut activity_visible = use_signal(|| true);
    use_hook(move || {
        spawn(async move {
            let mut updates = crate::features::runtime::lifecycle::subscribe_visibility();
            while let Some(visible) = updates.next().await {
                activity_visible.set(visible);
            }
        })
    });

    let ringing_playback = playback.clone();
    let ringing_call = direct_call.clone();
    let mut incoming_ring_state = use_signal(IncomingCallRingState::default);
    use_effect(move || {
        let incoming_call_id = ringing_call.incoming_call().map(|call| call.call_id);
        let ignored_call_id = incoming_toast_state().ignored_call_id;
        let ignored = incoming_call_id.is_some() && incoming_call_id == ignored_call_id;
        let mut state = incoming_ring_state.peek().cloned();
        let transition = state.sync(incoming_call_id.as_deref(), activity_visible(), ignored);

        if transition.stop {
            ringing_playback.stop_incoming_call_ring();
            debug!("stopped incoming direct-call ringtone");
        }
        if let Some(call_id) = transition.start {
            ringing_playback.start_incoming_call_ring();
            info!(%call_id, "started incoming direct-call ringtone");
        }
        incoming_ring_state.set(state);
    });
    let cleanup_playback = playback.clone();
    use_drop(move || cleanup_playback.stop_incoming_call_ring());

    let incoming_sound_call = direct_call.clone();
    let mut last_incoming_call_id = use_signal(|| None::<String>);
    let mut last_notification_visibility = use_signal(|| true);
    use_effect(move || {
        let incoming_call = incoming_sound_call.incoming_call();
        let incoming_call_id = incoming_call.as_ref().map(|call| call.call_id.clone());
        let previous_call_id = last_incoming_call_id.peek().clone();
        let was_visible = *last_notification_visibility.peek();
        let visible = activity_visible();
        let ignored = incoming_call_id.is_some()
            && incoming_call_id == incoming_toast_state().ignored_call_id;

        if let Some(previous_call_id) = previous_call_id.as_ref()
            && incoming_call_id.as_ref() != Some(previous_call_id)
        {
            clear_incoming_call_notification(previous_call_id.clone());
        }
        if visible
            && !was_visible
            && let Some(call_id) = incoming_call_id.as_ref()
        {
            clear_incoming_call_notification(call_id.clone());
        }
        if should_show_incoming_call_notification(
            incoming_call_id.as_deref(),
            previous_call_id.as_deref(),
            was_visible,
            visible,
            ignored,
        ) && let Some(call) = incoming_call
        {
            show_incoming_call_notification(
                call.call_id,
                call.conversation_id,
                call.caller_nickname,
            );
        }
        last_incoming_call_id.set(incoming_call_id);
        last_notification_visibility.set(visible);
    });

    let incoming_toast_call = direct_call.clone();
    use_effect(move || {
        let incoming_call = incoming_toast_call.incoming_call();
        let busy = incoming_toast_call.busy();
        let error = incoming_call
            .as_ref()
            .and_then(|call| incoming_toast_call.error_for_conversation(&call.conversation_id));
        let mut state = incoming_toast_state.peek().cloned();
        let transition = state.sync(incoming_call.as_ref().map(|call| call.call_id.as_str()));

        if let Some(previous_key) = transition.dismiss {
            toasts.dismiss_custom(&previous_key);
        }
        if let Some(call) = incoming_call
            && state.should_display(&call.call_id)
        {
            let call_id = call.call_id.clone();
            let key = incoming_toast_key(&call_id);
            let sender = incoming_toast_action_sender.clone();
            if transition.show {
                debug!(call_id = %call_id, "showing incoming direct call toast");
            }
            // В родительский ToastProvider передаём только данные и канал действий.
            // Dioxus Signals остаются в области DirectCallProvider.
            toasts.custom(key, move || {
                let action_sender = sender.clone();
                let action_id = call_id.clone();
                rsx! {
                    DirectCallToast {
                        call: call.clone(),
                        busy,
                        error: error.clone(),
                        on_action: move |action: DirectCallToastAction| {
                            if action_sender.unbounded_send((action_id.clone(), action)).is_err() {
                                debug!(call_id = %action_id, "incoming direct call toast action receiver closed");
                            }
                        },
                    }
                }
            });
        }

        incoming_toast_state.set(state);
    });

    rsx! {
        ActiveVoiceNotificationControls {}
        {children}
    }
}

/// Текущий вызов, для которого запущен внутренний рингтон.
#[derive(Clone, Default)]
struct IncomingCallRingState {
    ringing_call_id: Option<String>,
}

/// Действия над аудио после смены вызова, игнорирования или видимости.
#[derive(Default)]
struct IncomingCallRingTransition {
    stop: bool,
    start: Option<String>,
}

impl IncomingCallRingState {
    fn sync(
        &mut self,
        incoming_call_id: Option<&str>,
        visible: bool,
        ignored: bool,
    ) -> IncomingCallRingTransition {
        let desired_call_id = incoming_call_id.filter(|_| visible && !ignored);
        if self.ringing_call_id.as_deref() == desired_call_id {
            return IncomingCallRingTransition::default();
        }

        let stop = self.ringing_call_id.take().is_some();
        self.ringing_call_id = desired_call_id.map(str::to_owned);
        IncomingCallRingTransition {
            stop,
            start: self.ringing_call_id.clone(),
        }
    }
}

/// Локальное состояние показа и игнорирования входящего звонка.
#[derive(Clone, Default)]
struct IncomingCallToastState {
    shown_call_id: Option<String>,
    ignored_call_id: Option<String>,
}

/// Изменения, которые нужно применить к глобальным уведомлениям.
#[derive(Default)]
struct IncomingCallToastTransition {
    dismiss: Option<String>,
    show: bool,
}

impl IncomingCallToastState {
    fn sync(&mut self, incoming_call_id: Option<&str>) -> IncomingCallToastTransition {
        if self.shown_call_id.as_deref() == incoming_call_id {
            return IncomingCallToastTransition::default();
        }

        let dismiss = self
            .shown_call_id
            .take()
            .map(|call_id| incoming_toast_key(&call_id));
        self.shown_call_id = incoming_call_id.map(str::to_owned);
        let show = incoming_call_id
            .is_some_and(|call_id| self.ignored_call_id.as_deref() != Some(call_id));

        IncomingCallToastTransition { dismiss, show }
    }

    /// Разрешает показ или обновление уведомления только для текущего неигнорированного звонка.
    fn should_display(&self, call_id: &str) -> bool {
        self.shown_call_id.as_deref() == Some(call_id)
            && self.ignored_call_id.as_deref() != Some(call_id)
    }

    fn ignore(&mut self, call_id: &str) -> bool {
        if self.shown_call_id.as_deref() != Some(call_id) {
            return false;
        }
        self.ignored_call_id = Some(call_id.to_owned());
        true
    }
}

fn should_show_incoming_call_notification(
    incoming_call_id: Option<&str>,
    previous_call_id: Option<&str>,
    was_visible: bool,
    visible: bool,
    ignored: bool,
) -> bool {
    incoming_call_id.is_some()
        && !ignored
        && (incoming_call_id != previous_call_id || (was_visible && !visible))
}

fn incoming_toast_key(call_id: &str) -> String {
    format!("incoming:{call_id}")
}

#[cfg(test)]
mod tests;

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
