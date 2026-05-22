//! Toast notification context provider.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const TOAST_TTL_MS: u32 = 4_200;
const TOAST_EXIT_MS: u32 = 180;
const MAX_TOASTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToastKind {
    /// Successful operation feedback.
    Success,
    /// Recoverable warning feedback.
    Warning,
    /// Failed operation feedback.
    Error,
    /// Neutral informational feedback.
    Info,
}

impl ToastKind {
    fn accent_class(self) -> &'static str {
        match self {
            Self::Success => "bg-emerald-400",
            Self::Warning => "bg-amber-400",
            Self::Error => "bg-red-400",
            Self::Info => "bg-blue-400",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Success => "Готово",
            Self::Warning => "Проверьте",
            Self::Error => "Не получилось",
            Self::Info => "Информация",
        }
    }

    fn live_region(self) -> &'static str {
        match self {
            Self::Error | Self::Warning => "assertive",
            Self::Success | Self::Info => "polite",
        }
    }

    fn role(self) -> &'static str {
        match self {
            Self::Error | Self::Warning => "alert",
            Self::Success | Self::Info => "status",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Toast {
    id: u64,
    kind: ToastKind,
    message: String,
    exiting: bool,
}

/// Context handle used by features to show global toast notifications.
#[derive(Clone, Copy)]
pub(crate) struct ToastHandle {
    toasts: Signal<Vec<Toast>>,
    next_id: Signal<u64>,
}

impl ToastHandle {
    /// Shows a success toast.
    pub(crate) fn success(&self, message: impl Into<String>) {
        self.push(ToastKind::Success, message.into());
    }

    /// Shows a warning toast.
    pub(crate) fn warning(&self, message: impl Into<String>) {
        self.push(ToastKind::Warning, message.into());
    }

    /// Shows an error toast.
    pub(crate) fn error(&self, message: impl Into<String>) {
        self.push(ToastKind::Error, message.into());
    }

    /// Shows an informational toast.
    #[allow(dead_code)]
    pub(crate) fn info(&self, message: impl Into<String>) {
        self.push(ToastKind::Info, message.into());
    }

    fn push(&self, kind: ToastKind, message: String) {
        let mut next_id = self.next_id;
        let id = next_id() + 1;
        next_id.set(id);

        let mut toasts = self.toasts;
        let mut next_toasts = toasts();
        next_toasts.push(Toast {
            id,
            kind,
            message,
            exiting: false,
        });
        if next_toasts.len() > MAX_TOASTS {
            let overflow = next_toasts.len() - MAX_TOASTS;
            next_toasts.drain(0..overflow);
        }
        toasts.set(next_toasts);
        debug!(toast_id = id, kind = ?kind, "queued toast notification");

        spawn(async move {
            TimeoutFuture::new(TOAST_TTL_MS).await;
            begin_dismiss_toast(&mut toasts, id);
        });
    }
}

/// Provides global toast notifications to the client.
#[component]
pub(crate) fn ToastProvider(children: Element) -> Element {
    let mut toasts = use_signal(Vec::<Toast>::new);
    let next_id = use_signal(|| 0_u64);
    let handle = ToastHandle { toasts, next_id };
    use_context_provider(move || handle);

    rsx! {
        {children}
        div {
            class: "pointer-events-none fixed inset-x-0 top-3 z-[1100] flex flex-col items-center gap-2 px-3 sm:inset-x-auto sm:right-4 sm:top-4 sm:w-[360px] sm:items-stretch sm:px-0",
            for toast in toasts() {
                article {
                    key: "{toast.id}",
                    role: toast.kind.role(),
                    "aria-live": toast.kind.live_region(),
                    class: toast_class(toast.exiting),
                    div { class: "mt-1 flex h-5 w-5 shrink-0 items-center justify-center",
                        span { class: "h-2.5 w-2.5 rounded-full {toast.kind.accent_class()}" }
                    }
                    div { class: "min-w-0 flex-1 space-y-0.5",
                        p { class: "text-[12px] font-semibold leading-4 text-zinc-100", "{toast.kind.label()}" }
                        p { class: "break-words text-[13px] leading-5 text-zinc-300", "{toast.message}" }
                    }
                    button {
                        r#type: "button",
                        "aria-label": "Закрыть уведомление",
                        class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[18px] leading-none text-zinc-500 transition hover:bg-white/5 hover:text-zinc-100",
                        onclick: move |_| begin_dismiss_toast(&mut toasts, toast.id),
                        "×"
                    }
                    span { class: "toast-progress absolute bottom-0 left-0 h-px {toast.kind.accent_class()}" }
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

fn begin_dismiss_toast(toasts: &mut Signal<Vec<Toast>>, id: u64) {
    let mut next_toasts = toasts();
    let Some(toast) = next_toasts.iter_mut().find(|toast| toast.id == id) else {
        return;
    };
    if toast.exiting {
        return;
    }

    toast.exiting = true;
    toasts.set(next_toasts);
    debug!(toast_id = id, "dismissing toast notification");

    let mut toasts = *toasts;
    spawn(async move {
        TimeoutFuture::new(TOAST_EXIT_MS).await;
        remove_toast(&mut toasts, id);
    });
}

fn remove_toast(toasts: &mut Signal<Vec<Toast>>, id: u64) {
    let mut next_toasts = toasts();
    let before = next_toasts.len();
    next_toasts.retain(|toast| toast.id != id);
    if before == next_toasts.len() {
        return;
    }

    toasts.set(next_toasts);
    debug!(toast_id = id, "removed toast notification");
}
