//! Ленивое пакетное разрешение ролей авторов, видимых в истории и закреплениях.

use std::collections::HashMap;
use std::time::Duration;

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;
use crate::features::runtime::sleep_duration;
use crate::features::server_registry::ServerRegistry;

use super::realtime;
use super::room_pins::RoomPinsState;

#[derive(Clone)]
pub(super) struct AuthorRolesTarget {
    pub(super) realtime: RealtimeHandle,
    pub(super) registry: ServerRegistry,
    pub(super) server_id: String,
    pub(super) room_id: String,
}

/// Собирает неизвестных авторов, выдерживает 100 мс и запрашивает не больше 100 за раз.
pub(super) fn use_author_role_resolution(
    target: AuthorRolesTarget,
    messages: Signal<Vec<TextChatMessage>>,
    pins: RoomPinsState,
) {
    let mut busy = use_signal(|| false);
    let mut refresh_tick = use_signal(|| 0_u64);
    use_future(move || async move {
        loop {
            sleep_duration(Duration::from_secs(30)).await;
            *refresh_tick.write() += 1;
        }
    });
    use_effect(move || {
        // Проверяем TTL даже в тихой комнате, без лишних сетевых запросов.
        let _tick = refresh_tick();
        // Подписываемся на новые сообщения, закрепления и инвалидации ролей.
        let mut authors = messages
            .read()
            .iter()
            .map(|message| message.author_user_id.clone())
            .collect::<Vec<_>>();
        authors.extend(pins.author_ids());
        let _generation = target.registry.author_role_generation(&target.server_id);
        let unresolved = target
            .registry
            .unresolved_authors(&target.server_id, &authors);
        if unresolved.is_empty() || busy() {
            return;
        }
        busy.set(true);
        let target = target.clone();
        spawn(async move {
            sleep_duration(Duration::from_millis(100)).await;
            let mut authors = messages
                .read()
                .iter()
                .map(|message| message.author_user_id.clone())
                .collect::<Vec<_>>();
            authors.extend(pins.author_ids());
            let missing = target
                .registry
                .unresolved_authors(&target.server_id, &authors);
            for batch in missing.chunks(100) {
                let requested = batch.to_vec();
                let generation = target.registry.author_role_generation(&target.server_id);
                match realtime::resolve_author_roles(
                    &target.realtime,
                    target.server_id.clone(),
                    target.room_id.clone(),
                    requested.clone(),
                )
                .await
                {
                    Ok(response) if response.server_id == target.server_id => {
                        target.registry.absorb_author_roles(
                            &target.server_id,
                            generation,
                            &requested,
                            &response.author_role_ids,
                        );
                    }
                    Ok(_) => {
                        warn!(server_id = %target.server_id, "received author roles for another server");
                        break;
                    }
                    Err(error) => {
                        warn!(server_id = %target.server_id, room_id = %target.room_id,
                            %error, "failed to resolve text chat author roles");
                        // Короткий отрицательный TTL предотвращает повторные запросы при ошибке.
                        target.registry.absorb_author_roles(
                            &target.server_id,
                            generation,
                            &missing,
                            &HashMap::new(),
                        );
                        break;
                    }
                }
            }
            busy.set(false);
        });
    });
}
