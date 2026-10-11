//! Выбирает реализацию глобального ввода, не раскрывая platform API соседним модулям.

#[cfg(not(any(
    feature = "web",
    all(feature = "desktop", feature = "windows", target_os = "windows"),
    all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )
)))]
mod unsupported;
#[cfg(feature = "web")]
mod web;
#[cfg(all(
    feature = "desktop",
    feature = "windows",
    target_os = "windows",
    not(feature = "web")
))]
mod windows;

#[cfg(not(any(
    feature = "web",
    all(feature = "desktop", feature = "windows", target_os = "windows"),
    all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )
)))]
pub(in crate::features::microphone) use unsupported::{
    Monitor, key_label, record_binding, supported, unsupported_reason,
};
#[cfg(feature = "web")]
pub(in crate::features::microphone) use web::{
    key_label, record_binding, supported, unsupported_reason,
};
#[cfg(all(
    feature = "desktop",
    feature = "windows",
    target_os = "windows",
    not(feature = "web")
))]
pub(in crate::features::microphone) use windows::{
    Monitor, key_label, record_binding, supported, unsupported_reason,
};

#[cfg(all(
    feature = "desktop",
    feature = "linux",
    target_os = "linux",
    not(feature = "web")
))]
mod linux;
#[cfg(all(
    feature = "desktop",
    feature = "linux",
    target_os = "linux",
    not(feature = "web")
))]
pub(crate) use linux::binding::PushToTalkKey;
#[cfg(all(
    feature = "desktop",
    feature = "linux",
    target_os = "linux",
    not(feature = "web")
))]
pub(in crate::features::microphone) use linux::{
    Monitor, record_binding, supported, unsupported_reason,
};
#[cfg(not(all(
    feature = "desktop",
    feature = "linux",
    target_os = "linux",
    not(feature = "web")
)))]
mod legacy_key;
#[cfg(not(all(
    feature = "desktop",
    feature = "linux",
    target_os = "linux",
    not(feature = "web")
)))]
pub(crate) use legacy_key::PushToTalkKey;

/// Проверяет готовность глобального ввода без блокировки UI.
pub(in crate::features::microphone) async fn availability() -> bool {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    {
        linux::availability().await
    }
    #[cfg(not(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )))]
    {
        supported()
    }
}

/// Передаёт обновления Linux portal; платформы с прежним вводом не создают подписку.
pub(in crate::features::microphone) async fn observe_updates(on_update: impl FnMut(super::Update)) {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    linux::observe_updates(on_update).await;
    #[cfg(not(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )))]
    {
        let _ = on_update;
        std::future::pending::<()>().await;
    }
}

/// Подтверждает закрытие системного окна, когда portal не сообщает результат ConfigureShortcuts.
pub(in crate::features::microphone) fn finish_assignment() {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    linux::finish_assignment();
}

/// Запрашивает Linux probe; остальные платформы сохраняют прежнюю доступность.
pub(in crate::features::microphone) fn retry_availability() {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    linux::retry_availability();
}

/// Linux Monitor следит за постоянным shortcut ID; hooks других платформ требуют перезапуска.
pub(in crate::features::microphone) fn binding_requires_capture_restart() -> bool {
    !cfg!(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))
}

/// Освобождает platform worker при завершении microphone provider.
pub(in crate::features::microphone) fn shutdown() {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    linux::shutdown();
}

/// Сохраняет закрытый PTT-режим даже при временной недоступности Linux portal.
pub(in crate::features::microphone) fn restore_saved_mode() -> bool {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    {
        true
    }
    #[cfg(not(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )))]
    {
        supported()
    }
}

/// Описывает допустимый способ назначения для текущей платформы.
pub(in crate::features::microphone) fn binding_prompt() -> &'static str {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    {
        "Выберите сочетание в системном окне. После закрытия окна нажмите «Готово» или «Отменить» здесь, если выбор не завершился автоматически. Передача голоса приостановлена на время назначения."
    }
    #[cfg(not(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )))]
    {
        "Нажмите и отпустите клавишу или кнопку мыши, включая Mouse4 и Mouse5. Передача голоса приостановлена на время назначения."
    }
}

/// Описывает режим без обещаний неподдерживаемых платформой кнопок.
pub(in crate::features::microphone) fn mode_description() -> &'static str {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    {
        "Говорите, удерживая выбранное системное сочетание."
    }
    #[cfg(not(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )))]
    {
        "Говорите, удерживая кнопку клавиатуры или мыши."
    }
}

/// Возвращает текст ожидания системного назначения.
pub(in crate::features::microphone) fn waiting_label() -> &'static str {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    {
        "Ожидаем выбор…"
    }
    #[cfg(not(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )))]
    {
        "Ожидаем кнопку…"
    }
}

/// Объясняет удержание с учётом гарантий платформенного источника событий.
pub(in crate::features::microphone) fn hold_description() -> &'static str {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    {
        "Удерживайте выбранное сочетание, чтобы говорить. Отпустите, чтобы прекратить передачу."
    }
    #[cfg(not(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )))]
    {
        "Удерживайте выбранную кнопку, чтобы говорить. Отпустите, чтобы прекратить передачу. Кнопка также продолжит работать в других приложениях."
    }
}

/// Разделяет Linux portal metadata и прежнюю Windows-настройку.
pub(in crate::features::microphone) const BINDING_STORAGE_KEY: &str = if cfg!(all(
    feature = "desktop",
    feature = "linux",
    target_os = "linux",
    not(feature = "web")
)) {
    "cheenhub.microphone.portal_push_to_talk"
} else {
    "cheenhub.microphone.push_to_talk_key"
};

/// Отзывает Linux capture lease независимо от отложенного освобождения queued frames.
#[cfg(all(feature = "linux", not(feature = "web")))]
pub(in crate::features::microphone) fn release_capture_monitor(monitor: &Monitor) {
    #[cfg(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    ))]
    monitor.stop();
    #[cfg(not(all(
        feature = "desktop",
        feature = "linux",
        target_os = "linux",
        not(feature = "web")
    )))]
    let _ = monitor;
}
