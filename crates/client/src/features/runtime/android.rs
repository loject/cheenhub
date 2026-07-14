//! Контракты между Rust feature-модулями и Android Activity/Service слоем.

#![cfg(target_os = "android")]

use std::fmt;

mod bridge;

pub(crate) use bridge::{android_bridge, take_media_projection_grant};

/// Runtime-разрешение, запрашиваемое у Android Activity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AndroidPermission {
    /// Доступ к микрофону.
    RecordAudio,
    /// Доступ к камере.
    Camera,
    /// Разрешение Android 13+ на показ системных уведомлений.
    PostNotifications,
}

/// Результат Android runtime permission request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PermissionResult {
    /// Разрешение выдано.
    Granted,
    /// Разрешение отклонено, но его можно запросить повторно.
    Denied,
    /// Разрешение отклонено без возможности повторного системного запроса.
    DeniedPermanently,
}

/// Тип foreground service, владеющего активной media-сессией.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ForegroundServiceKind {
    /// Захват и воспроизведение голоса.
    Voice,
    /// Захват камеры.
    Camera,
    /// Демонстрация экрана через MediaProjection.
    MediaProjection,
}

/// Непрозрачный идентификатор выданного Android MediaProjection consent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct MediaProjectionGrant(pub(crate) u64);

/// Идентификаторы Android-установки, необходимые для регистрации push-доставки.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AndroidPushInstallation {
    /// Стабильный локальный UUID установки приложения.
    pub(crate) installation_id: String,
    /// Непрозрачный токен Firebase Cloud Messaging.
    pub(crate) token: String,
}

/// Ошибка обращения к Android platform bridge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AndroidBridgeError(String);

impl AndroidBridgeError {
    /// Создаёт ошибку с безопасным для журнала описанием.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for AndroidBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AndroidBridgeError {}

/// Activity/Service bridge, устанавливаемый Android bootstrap-кодом.
pub(crate) trait AndroidBridge: Send + Sync {
    /// Запрашивает runtime-разрешение и однократно вызывает callback с результатом.
    fn request_permission(
        &self,
        permission: AndroidPermission,
        callback: Box<dyn FnOnce(Result<PermissionResult, AndroidBridgeError>) + Send>,
    ) -> Result<(), AndroidBridgeError>;

    /// Открывает системный MediaProjection consent dialog.
    fn request_media_projection(
        &self,
        callback: Box<dyn FnOnce(Result<Option<MediaProjectionGrant>, AndroidBridgeError>) + Send>,
    ) -> Result<(), AndroidBridgeError>;

    /// Запускает foreground service до открытия защищённого media-ресурса.
    fn start_foreground_service(
        &self,
        kind: ForegroundServiceKind,
    ) -> Result<(), AndroidBridgeError>;

    /// Останавливает foreground service после закрытия media-ресурса.
    fn stop_foreground_service(
        &self,
        kind: ForegroundServiceKind,
    ) -> Result<(), AndroidBridgeError>;

    /// Запрашивает текущий UUID установки и FCM token у Android-слоя.
    fn request_push_installation(
        &self,
        callback: Box<dyn FnOnce(Result<AndroidPushInstallation, AndroidBridgeError>) + Send>,
    ) -> Result<(), AndroidBridgeError>;

    /// Возвращает и удаляет отложенный переход в личный диалог из уведомления.
    fn take_pending_direct_message_conversation_id(
        &self,
        callback: Box<dyn FnOnce(Result<Option<String>, AndroidBridgeError>) + Send>,
    ) -> Result<(), AndroidBridgeError>;

    /// Передаёт Android-слою открытый личный диалог для подавления лишнего уведомления.
    fn set_active_direct_message_conversation(
        &self,
        conversation_id: Option<String>,
    ) -> Result<(), AndroidBridgeError>;

    /// Удаляет системное уведомление и локальную историю прочитанного диалога.
    fn clear_direct_message_notification(
        &self,
        conversation_id: String,
    ) -> Result<(), AndroidBridgeError>;
}
