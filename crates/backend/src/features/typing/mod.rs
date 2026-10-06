//! Индикатор набора сообщения в комнатах и личных диалогах.
//!
//! Фича хранит только оперативное состояние «кто печатает» в памяти процесса и
//! не создает записей в базе: событие набора полностью восстанавливается из
//! входящего ввода, поэтому терять его при перезапуске нечего.
//!
//! Границы ответственности:
//! - [`infrastructure`] хранит записи набора с временем жизни и чистит их по
//!   потоку realtime, а также ограничивает частоту продлений.
//! - [`application`] проверяет права доступа, вещает события и отвечает на
//!   запрос снимка.
//! - Контракты и коду событий для комнат и личных диалогов остаются в
//!   `text_chat` и `social`: этот модуль только обслуживает их состояние.

pub(crate) mod application;
pub(crate) mod infrastructure;

pub(crate) use application::{
    direct_message_typing_snapshot, disconnect_realtime_stream, parse_conversation_id,
    parse_room_target, room_typing_snapshot, send_rejection_for as send_typing_rejection,
    spawn_expiry_sweeper, start_direct_message_typing, start_room_typing,
    stop_direct_message_typing, stop_room_typing,
};
pub(crate) use infrastructure::{InMemoryTypingStore, TypingAuthorEntry, TypingTarget};
