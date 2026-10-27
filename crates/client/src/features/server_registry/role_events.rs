//! Обновляет адресный кэш цветов авторов по событиям текстового realtime.

use cheenhub_contracts::realtime::{
    AuthorRolesChanged, RealtimeEnvelope, RealtimeKind, RealtimeModule, TextChatKind,
};
use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::realtime::RealtimeHandle;

use super::ServerRegistry;

/// Подписка живёт вместе с реестром текущей авторизованной оболочки.
pub(super) fn use_author_role_events(registry: ServerRegistry) {
    let realtime = use_context::<RealtimeHandle>();
    use_future(move || {
        let realtime = realtime.clone();
        async move {
            let mut events = realtime.subscribe_events();
            while let Some(envelope) = events.next().await {
                if let Some(change) = decode_author_role_change(envelope) {
                    apply_author_role_change(registry, change);
                }
            }
        }
    });
}

fn decode_author_role_change(envelope: RealtimeEnvelope) -> Option<AuthorRolesChanged> {
    if envelope.module != RealtimeModule::TextChat
        || envelope.kind != RealtimeKind::TextChat(TextChatKind::AuthorRolesChanged)
    {
        return None;
    }
    match serde_json::from_value(envelope.payload) {
        Ok(change) => Some(change),
        Err(error) => {
            warn!(%error, "invalid AuthorRolesChanged realtime payload");
            None
        }
    }
}

fn apply_author_role_change(registry: ServerRegistry, change: AuthorRolesChanged) {
    if let Some(roles) = change.roles.as_deref() {
        registry.set_roles(&change.server_id, roles);
    } else if let Some(user_id) = change.user_id.as_deref() {
        registry.invalidate_author_roles(&change.server_id, user_id);
    } else {
        registry.invalidate_server_author_roles(&change.server_id);
    }
}

#[cfg(test)]
mod tests;
