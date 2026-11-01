//! Глобальный signaling и уведомления приглашений в серверные голосовые комнаты.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::AudioPlaybackHandle;
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};
use crate::features::toast::ToastHandle;

use super::direct_call_provider::DirectCallRingStatus;
use super::room_invite_realtime;
use super::room_invite_state::{RoomInviteHandle, RoomInviteSignals};
use super::room_invite_toast::{RoomInviteToast, RoomInviteToastAction};
use super::state::VoiceConnectionHandle;

/// Предоставляет приглашения всем вложенным экранам и показывает глобальные уведомления.
#[component]
pub(super) fn RoomInviteProvider(children: Element) -> Element {
    let user_id = use_context::<CurrentUserContext>().require_user().id;
    let realtime = use_context::<RealtimeHandle>();
    let voice = use_context::<VoiceConnectionHandle>();
    let direct_ring = use_context::<DirectCallRingStatus>();
    let playback = use_context::<AudioPlaybackHandle>();
    let toasts = use_context::<ToastHandle>();
    let invites = use_signal(Vec::new);
    let pending_deadlines = use_signal(HashMap::new);
    let ignored = use_signal(HashSet::new);
    let candidates = use_signal(HashMap::new);
    let recent_results = use_signal(HashMap::new);
    let terminal_cleanup_deadlines = use_signal(HashMap::new);
    let busy = use_signal(|| false);
    let handle = RoomInviteHandle::new(
        RoomInviteSignals {
            invites,
            pending_deadlines,
            ignored_ids: ignored,
            candidates,
            recent_results,
            terminal_cleanup_deadlines,
            busy,
        },
        realtime.clone(),
        voice,
        toasts,
        user_id.clone(),
    );
    let context = handle.clone();
    use_context_provider(move || context.clone());

    // Действия обрабатываются в scope владельца сигналов; ToastProvider получает лишь канал.
    let action_handle = handle.clone();
    let action_sender = use_hook(move || {
        let (sender, mut receiver) = mpsc::unbounded::<(String, RoomInviteToastAction)>();
        spawn(async move {
            while let Some((invite_id, action)) = receiver.next().await {
                if !action_handle
                    .incoming()
                    .is_some_and(|invite| invite.invite_id == invite_id)
                {
                    continue;
                }
                match action {
                    RoomInviteToastAction::Accept => action_handle.accept(&invite_id),
                    RoomInviteToastAction::Decline => action_handle.decline(&invite_id),
                    RoomInviteToastAction::Ignore => action_handle.ignore(&invite_id),
                }
            }
        });
        sender
    });

    let event_realtime = realtime.clone();
    let event_handle = handle.clone();
    let event_toasts = toasts;
    let event_user_id = user_id;
    use_hook(move || {
        spawn(async move {
            let mut events = room_invite_realtime::subscribe(&event_realtime);
            while let Some(event) = events.next().await {
                let invite = event.invite.clone();
                if invite.caller_user_id == event_user_id {
                    let previous = event_handle
                        .outgoing_for_room(&invite.server_id, &invite.room_id)
                        .into_iter()
                        .find(|existing| existing.invite_id == invite.invite_id)
                        .map(|existing| existing.status);
                    if previous != Some(invite.status) {
                        use cheenhub_contracts::realtime::VoiceRoomInviteStatus;
                        match invite.status {
                            VoiceRoomInviteStatus::Accepted => {}
                            VoiceRoomInviteStatus::Declined => {
                                event_toasts.info("Приглашение отклонено.")
                            }
                            VoiceRoomInviteStatus::Expired => {
                                event_toasts.info("Время приглашения истекло.")
                            }
                            VoiceRoomInviteStatus::Cancelled | VoiceRoomInviteStatus::Pending => {}
                        }
                    }
                }
                event_handle.apply_event(event);
            }
        })
    });

    let recovery_realtime = realtime.clone();
    let recovery_handle = handle.clone();
    use_hook(move || {
        spawn(async move {
            let mut statuses = recovery_realtime.subscribe_connection_status();
            while let Some(status) = statuses.next().await {
                if matches!(status, RealtimeConnectionStatus::Connected(_)) {
                    recovery_handle.recover();
                }
            }
        })
    });

    // Приоритет у личного звонка; приглашение не должно заглушить его рингтон.
    let ring_handle = handle.clone();
    let ring_playback = playback.clone();
    let mut ringing_id = use_signal(|| None::<String>);
    use_effect(move || {
        let dm_ringing = direct_ring.is_ringing();
        let desired = desired_room_invite_ring(
            ring_handle.incoming().map(|invite| invite.invite_id),
            dm_ringing,
        );
        let previous = ringing_id.peek().clone();
        if previous == desired {
            return;
        }
        if previous.is_some() && !dm_ringing {
            ring_playback.stop_incoming_call_ring();
        }
        if let Some(invite_id) = desired.as_ref() {
            ring_playback.start_incoming_call_ring();
            debug!(%invite_id, "started incoming voice room invitation ringtone");
        }
        ringing_id.set(desired);
    });
    let cleanup_playback = playback.clone();
    use_drop(move || cleanup_playback.stop_incoming_call_ring());

    let toast_handle = handle.clone();
    let mut shown_toast_id = use_signal(|| None::<String>);
    use_effect(move || {
        let incoming = toast_handle.incoming();
        let next_id = incoming.as_ref().map(|invite| invite.invite_id.clone());
        let previous = shown_toast_id.peek().clone();
        if previous != next_id
            && let Some(previous_id) = previous
        {
            toasts.dismiss_custom(&toast_key(&previous_id));
        }
        if let Some(invite) = incoming {
            let invite_id = invite.invite_id.clone();
            let sender = action_sender.clone();
            let busy = toast_handle.busy();
            toasts.custom(toast_key(&invite_id), move || {
                let action_id = invite_id.clone();
                let action_sender = sender.clone();
                rsx! {
                    RoomInviteToast {
                        invite: invite.clone(),
                        busy,
                        on_action: move |action: RoomInviteToastAction| {
                            let _ = action_sender.unbounded_send((action_id.clone(), action));
                        },
                    }
                }
            });
        }
        shown_toast_id.set(next_id);
    });

    rsx! { {children} }
}

/// Уступает общий рингтон только звучащему личному звонку, а не проигнорированному.
fn desired_room_invite_ring(
    invite_id: Option<String>,
    direct_call_ringing: bool,
) -> Option<String> {
    invite_id.filter(|_| !direct_call_ringing)
}

fn toast_key(invite_id: &str) -> String {
    format!("voice-room-invite:{invite_id}")
}

#[cfg(test)]
mod tests;
