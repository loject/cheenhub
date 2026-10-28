//! Чтение единого списка друзей и собеседников через общий realtime-транспорт.

use cheenhub_contracts::realtime::{
    ListSocialEntries, RealtimeKind, RealtimeModule, SocialEntries, SocialKind,
};
use dioxus::prelude::warn;

use crate::features::realtime::RealtimeHandle;

/// Запрашивает страницу участников; курсор передаётся только с тем же поиском.
pub(crate) async fn list_social_entries(
    realtime: &RealtimeHandle,
    query: Option<String>,
    cursor: Option<String>,
) -> Result<SocialEntries, String> {
    let request = ListSocialEntries {
        query,
        cursor,
        limit: Some(50),
    };
    realtime
        .request(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::ListSocialEntries),
            request,
        )
        .await
        .map_err(|error| {
            warn!(%error, "unified social list realtime request failed");
            error.user_message().unwrap_or(
                "Не удалось загрузить список друзей и диалогов. Проверь соединение и повтори попытку.",
            ).to_owned()
        })
}
