//! Модули клиентских функций.

pub(crate) mod app;
pub(crate) mod application_focus;
pub(crate) mod application_update;
pub(crate) mod audio_playback;
pub(crate) mod auth;
pub(crate) mod autostart;
pub(crate) mod camera;
pub(crate) mod clipboard;
pub(crate) mod host_settings;
pub(crate) mod image_picker;
pub(crate) mod landing;
pub(crate) mod legal;
/// Единая форма ввода и отправки сообщений.
pub(crate) mod message_composer;
pub(crate) mod microphone;
pub(crate) mod network;
pub(crate) mod notifications;
pub(crate) mod pwa;
pub(crate) mod realtime;
pub(crate) mod runtime;
pub(crate) mod screen_share;
/// Метаданные серверов, роли и права текущего пользователя.
pub(crate) mod server_registry;
pub(crate) mod server_settings;
pub(crate) mod single_instance;
pub(crate) mod social;
pub(crate) mod system_tray;
pub(crate) mod text_chat;
pub(crate) mod toast;
pub(crate) mod user_settings;
pub(crate) mod video_encoding;
pub(crate) mod voice_chat;
