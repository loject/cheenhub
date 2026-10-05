//! Провайдер контекста toast-уведомлений.

use dioxus::prelude::*;

use super::update_available::UpdateAvailableToast;
use crate::features::application_focus::ApplicationFocusContext;
const MAX_TOASTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToastKind {
    /// Успешное завершение действия.
    Success,
    /// Предупреждение, которое можно исправить.
    Warning,
    /// Ошибка выполнения действия.
    Error,
    /// Нейтральное информационное сообщение.
    Info,
    /// Постоянное уведомление о доступном обновлении приложения.
    UpdateAvailable,
}

impl ToastKind {
    fn accent_class(self) -> &'static str {
        match self {
            Self::Success => "bg-emerald-400",
            Self::Warning => "bg-amber-400",
            Self::Error => "bg-red-400",
            Self::Info => "bg-blue-400",
            Self::UpdateAvailable => "bg-blue-400",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Success => "Готово",
            Self::Warning => "Проверьте",
            Self::Error => "Не получилось",
            Self::Info => "Информация",
            Self::UpdateAvailable => "Обновление доступно",
        }
    }

    fn live_region(self) -> &'static str {
        match self {
            Self::Error | Self::Warning => "assertive",
            Self::Success | Self::Info | Self::UpdateAvailable => "polite",
        }
    }

    fn role(self) -> &'static str {
        match self {
            Self::Error | Self::Warning => "alert",
            Self::Success | Self::Info | Self::UpdateAvailable => "status",
        }
    }

    fn persistent(self) -> bool {
        matches!(self, Self::UpdateAvailable)
    }
}

#[derive(Clone)]
enum ToastPayload {
    Message(String),
    UpdateAvailable(UpdateAvailableToast),
}

#[derive(Clone)]
pub(super) struct Toast {
    id: u64,
    kind: ToastKind,
    payload: ToastPayload,
    exiting: bool,
    hovered: bool,
    remaining_ms: u32,
    protected_until_focused: bool,
}

impl Toast {
    pub(super) fn id(&self) -> u64 {
        self.id
    }

    pub(super) fn timer_paused(&self, application_focused: bool) -> bool {
        self.hovered || !application_focused
    }

    pub(super) fn countdown_active(&self) -> bool {
        !self.kind.persistent() && !self.exiting
    }

    pub(super) fn begin_exit(&mut self) -> bool {
        if self.exiting {
            return false;
        }
        self.exiting = true;
        true
    }

    pub(super) fn set_hovered(&mut self, hovered: bool) -> bool {
        if self.hovered == hovered || self.exiting {
            return false;
        }
        self.hovered = hovered;
        true
    }

    pub(super) fn tick(&mut self, elapsed_ms: u32, application_focused: bool) -> bool {
        let paused = self.timer_paused(application_focused);
        super::timer::advance_remaining(&mut self.remaining_ms, elapsed_ms, paused)
    }

    pub(super) fn mark_focused_display(&mut self) -> bool {
        if !self.protected_until_focused {
            return false;
        }
        self.protected_until_focused = false;
        true
    }
}

/// Контекстный handle для показа глобальных toast-уведомлений.
#[derive(Clone, Copy)]
pub(crate) struct ToastHandle {
    toasts: Signal<Vec<Toast>>,
    next_id: Signal<u64>,
    application_focus: ApplicationFocusContext,
}

impl ToastHandle {
    /// Показывает сообщение об успешном действии.
    pub(crate) fn success(&self, message: impl Into<String>) {
        self.push_message(ToastKind::Success, message.into(), false);
    }

    /// Показывает предупреждение.
    pub(crate) fn warning(&self, message: impl Into<String>) {
        self.push_message(ToastKind::Warning, message.into(), false);
    }

    /// Показывает сообщение об ошибке.
    pub(crate) fn error(&self, message: impl Into<String>) {
        self.push_message(ToastKind::Error, message.into(), false);
    }

    /// Показывает причину завершения сеанса и сохраняет её до возврата фокуса.
    pub(crate) fn session_error(&self, message: impl Into<String>) {
        self.push(
            ToastKind::Error,
            ToastPayload::Message(message.into()),
            true,
        );
    }

    /// Показывает информационное сообщение.
    pub(crate) fn info(&self, message: impl Into<String>) {
        self.push_message(ToastKind::Info, message.into(), false);
    }

    /// Показывает постоянное уведомление о доступном обновлении.
    pub(crate) fn update_available(&self, mut toast: UpdateAvailableToast) {
        let mut toasts = self.toasts;
        let mut current = toasts.peek().clone();
        if let Some(existing) = current.iter_mut().find(|item| {
            matches!(&item.payload, ToastPayload::UpdateAvailable(update)
                if update.update_version == toast.update_version)
                && !item.exiting
        }) {
            if let ToastPayload::UpdateAvailable(previous) = &existing.payload {
                toast.selected_deferral_value = previous.selected_deferral_value.clone();
            }
            existing.payload = ToastPayload::UpdateAvailable(toast);
            toasts.set(current);
            return;
        }
        self.push(
            ToastKind::UpdateAvailable,
            ToastPayload::UpdateAvailable(toast),
            false,
        );
    }

    fn push_message(&self, kind: ToastKind, message: String, protected_until_focused: bool) {
        self.push(
            kind,
            ToastPayload::Message(message),
            protected_until_focused,
        );
    }

    fn push(&self, kind: ToastKind, payload: ToastPayload, protected_until_focused: bool) {
        let mut next_id = self.next_id;
        let id = *next_id.peek() + 1;
        next_id.set(id);

        let mut toasts = self.toasts;
        let mut next_toasts = toasts.peek().clone();
        if kind == ToastKind::UpdateAvailable {
            next_toasts.retain(|toast| toast.kind != ToastKind::UpdateAvailable);
        }
        next_toasts.push(Toast {
            id,
            kind,
            payload,
            exiting: false,
            hovered: false,
            remaining_ms: super::timer::TOAST_TTL_MS,
            protected_until_focused: protected_until_focused
                && !self.application_focus.is_focused(),
        });
        while next_toasts.len() > MAX_TOASTS {
            let Some(index) = next_toasts
                .iter()
                .position(|toast| !toast.protected_until_focused && !toast.kind.persistent())
            else {
                break;
            };
            next_toasts.remove(index);
        }
        toasts.set(next_toasts);
        debug!(toast_id = id, kind = ?kind, "queued toast notification");
    }
}

/// Предоставляет клиенту глобальные toast-уведомления.
#[component]
pub(crate) fn ToastProvider(children: Element) -> Element {
    let toasts = use_signal(Vec::<Toast>::new);
    let next_id = use_signal(|| 0_u64);
    let application_focus = use_context::<ApplicationFocusContext>();
    use_hook(move || {
        super::timer::spawn_scheduler_task(super::timer::run_toast_scheduler(
            toasts,
            application_focus,
        ))
    });
    let handle = ToastHandle {
        toasts,
        next_id,
        application_focus,
    };
    use_context_provider(move || handle);

    rsx! {
        {children}
        div {
            class: "pointer-events-none fixed inset-x-0 top-3 z-[1100] flex flex-col items-center gap-2 px-3 sm:inset-x-auto sm:right-4 sm:top-4 sm:w-[420px] sm:items-stretch sm:px-0",
            for toast in toasts() {
                {render_toast(toast, toasts, application_focus.is_focused())}
            }
        }
    }
}

fn render_toast(toast: Toast, toasts: Signal<Vec<Toast>>, application_focused: bool) -> Element {
    match toast.payload.clone() {
        ToastPayload::Message(message) => {
            render_message_toast(toast, message, toasts, application_focused)
        }
        ToastPayload::UpdateAvailable(update) => {
            update_view::render_update_available_toast(toast, update, toasts)
        }
    }
}

fn render_message_toast(
    toast: Toast,
    message: String,
    mut toasts: Signal<Vec<Toast>>,
    _application_focused: bool,
) -> Element {
    let progress_scale = toast.remaining_ms as f64 / super::timer::TOAST_TTL_MS as f64;
    rsx! {
        article {
            key: "{toast.id}",
            role: toast.kind.role(),
            "aria-live": toast.kind.live_region(),
            class: toast_class(toast.exiting),
            onmouseenter: move |_| super::timer::set_toast_hovered(&mut toasts, toast.id, true),
            onmouseleave: move |_| super::timer::set_toast_hovered(&mut toasts, toast.id, false),
            div { class: "mt-1 flex h-5 w-5 shrink-0 items-center justify-center",
                span { class: "h-2.5 w-2.5 rounded-full {toast.kind.accent_class()}" }
            }
            div { class: "min-w-0 flex-1 space-y-0.5",
                p { class: "text-[12px] font-semibold leading-4 text-zinc-100", "{toast.kind.label()}" }
                p { class: "break-words text-[13px] leading-5 text-zinc-300", "{message}" }
            }
            button {
                r#type: "button",
                "aria-label": "Закрыть уведомление",
                class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[18px] leading-none text-zinc-500 transition hover:bg-white/5 hover:text-zinc-100",
                onclick: move |_| super::timer::begin_dismiss_toast(&mut toasts, toast.id),
                "×"
            }
            if !toast.kind.persistent() {
                span {
                    class: "toast-progress absolute bottom-0 left-0 h-px {toast.kind.accent_class()}",
                    style: "transform: scaleX({progress_scale});"
                }
            }
        }
    }
}

fn toast_class(exiting: bool) -> &'static str {
    if exiting {
        "toast-item toast-item-exiting pointer-events-auto flex min-h-14 w-full max-w-[calc(100vw-1.5rem)] items-start gap-3 overflow-hidden rounded-lg border border-white/10 bg-zinc-950/95 px-3 py-3 text-zinc-100 shadow-[0_18px_50px_rgba(0,0,0,0.38)] backdrop-blur sm:max-w-none"
    } else {
        "toast-item pointer-events-auto flex min-h-14 w-full max-w-[calc(100vw-1.5rem)] items-start gap-3 overflow-hidden rounded-lg border border-white/10 bg-zinc-950/95 px-3 py-3 text-zinc-100 shadow-[0_18px_50px_rgba(0,0,0,0.38)] backdrop-blur sm:max-w-none"
    }
}

#[cfg(test)]
mod tests;

mod update_view;
