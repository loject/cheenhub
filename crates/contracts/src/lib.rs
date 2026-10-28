#![warn(missing_docs)]
//! Общие контракты клиента и сервера CheenHub.

/// Версии публичных юридических документов.
pub mod legal;

/// Общее представление присутствия пользователя.
pub mod presence;

/// Будущие модули контрактов REST API.
pub mod rest;

/// Будущие модули контрактов медиатранспорта.
pub mod media;

/// Модули контрактов realtime WebTransport.
pub mod realtime;

/// Пресеты разрешения и частоты исходящего видео.
pub mod video_presets;
