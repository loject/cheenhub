//! Контракты между Rust feature-модулями и Android Activity/Service слоем.

#![cfg(feature = "android")]

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
}
