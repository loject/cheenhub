//! Провайдер контекста toast-уведомлений.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::features::runtime::sleep_ms;

const TOAST_TTL_MS: u32 = 4_200;
const TOAST_EXIT_MS: u32 = 180;
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
pub(crate) struct UpdateAvailableToast {
    current_version: String,
    update_version: String,
    title: Option<String>,
    primary_label: String,
    primary_disabled: bool,
    deferral_options: Vec<UpdateToastDeferralOption>,
    selected_deferral_value: String,
    on_install: Rc<dyn Fn()>,
    on_quick_dismiss: Rc<dyn Fn()>,
    on_defer: Rc<dyn Fn(String)>,
}

impl UpdateAvailableToast {
    /// Создает данные toast-уведомления о доступном обновлении.
    pub(crate) fn new(
        content: UpdateAvailableToastContent,
        actions: UpdateAvailableToastActions,
    ) -> Self {
        Self {
            current_version: content.current_version,
            update_version: content.update_version,
            title: content.title,
            primary_label: content.primary_label,
            primary_disabled: content.primary_disabled,
            deferral_options: content.deferral_options,
            selected_deferral_value: content.default_deferral_value,
            on_install: actions.on_install,
            on_quick_dismiss: actions.on_quick_dismiss,
            on_defer: actions.on_defer,
        }
    }
}

/// Текстовые данные toast-уведомления о доступном обновлении.
pub(crate) struct UpdateAvailableToastContent {
    current_version: String,
    update_version: String,
    title: Option<String>,
    primary_label: String,
    primary_disabled: bool,
    deferral_options: Vec<UpdateToastDeferralOption>,
    default_deferral_value: String,
}

impl UpdateAvailableToastContent {
    /// Создает текстовые данные update-toast.
    pub(crate) fn new(
        current_version: impl Into<String>,
        update_version: impl Into<String>,
        title: Option<String>,
        primary_label: impl Into<String>,
        primary_disabled: bool,
        deferral_options: Vec<UpdateToastDeferralOption>,
        default_deferral_value: impl Into<String>,
    ) -> Self {
        Self {
            current_version: current_version.into(),
            update_version: update_version.into(),
            title,
            primary_label: primary_label.into(),
            primary_disabled,
            deferral_options,
            default_deferral_value: default_deferral_value.into(),
        }
    }
}

/// Действия toast-уведомления о доступном обновлении.
pub(crate) struct UpdateAvailableToastActions {
    on_install: Rc<dyn Fn()>,
    on_quick_dismiss: Rc<dyn Fn()>,
    on_defer: Rc<dyn Fn(String)>,
}

impl UpdateAvailableToastActions {
    /// Создает callbacks для update-toast.
    pub(crate) fn new(
        on_install: impl Fn() + 'static,
        on_quick_dismiss: impl Fn() + 'static,
        on_defer: impl Fn(String) + 'static,
    ) -> Self {
        Self {
            on_install: Rc::new(on_install),
            on_quick_dismiss: Rc::new(on_quick_dismiss),
            on_defer: Rc::new(on_defer),
        }
    }
}

/// Пункт выбора времени отсрочки внутри update-toast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UpdateToastDeferralOption {
    value: String,
    label: String,
}

impl UpdateToastDeferralOption {
    /// Создает пункт выбора времени отсрочки.
    pub(crate) fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

#[derive(Clone)]
enum ToastPayload {
    Message(String),
    UpdateAvailable(UpdateAvailableToast),
}

#[derive(Clone)]
struct Toast {
    id: u64,
    kind: ToastKind,
    payload: ToastPayload,
    exiting: bool,
}

/// Контекстный handle для показа глобальных toast-уведомлений.
#[derive(Clone, Copy)]
pub(crate) struct ToastHandle {
    toasts: Signal<Vec<Toast>>,
    next_id: Signal<u64>,
}

impl ToastHandle {
    /// Показывает сообщение об успешном действии.
    pub(crate) fn success(&self, message: impl Into<String>) {
        self.push_message(ToastKind::Success, message.into());
    }

    /// Показывает предупреждение.
    pub(crate) fn warning(&self, message: impl Into<String>) {
        self.push_message(ToastKind::Warning, message.into());
    }

    /// Показывает сообщение об ошибке.
    pub(crate) fn error(&self, message: impl Into<String>) {
        self.push_message(ToastKind::Error, message.into());
    }

    /// Показывает информационное сообщение.
    #[allow(dead_code)]
    pub(crate) fn info(&self, message: impl Into<String>) {
        self.push_message(ToastKind::Info, message.into());
    }

    /// Показывает постоянное уведомление о доступном обновлении.
    #[allow(dead_code)]
    pub(crate) fn update_available(&self, toast: UpdateAvailableToast) {
        self.push(
            ToastKind::UpdateAvailable,
            ToastPayload::UpdateAvailable(toast),
        );
    }

    fn push_message(&self, kind: ToastKind, message: String) {
        self.push(kind, ToastPayload::Message(message));
    }

    fn push(&self, kind: ToastKind, payload: ToastPayload) {
        let mut next_id = self.next_id;
        let id = next_id() + 1;
        next_id.set(id);

        let mut toasts = self.toasts;
        let mut next_toasts = toasts();
        if kind == ToastKind::UpdateAvailable {
            next_toasts.retain(|toast| toast.kind != ToastKind::UpdateAvailable);
        }
        next_toasts.push(Toast {
            id,
            kind,
            payload,
            exiting: false,
        });
        if next_toasts.len() > MAX_TOASTS {
            let overflow = next_toasts.len() - MAX_TOASTS;
            next_toasts.drain(0..overflow);
        }
        toasts.set(next_toasts);
        debug!(toast_id = id, kind = ?kind, "queued toast notification");

        if !kind.persistent() {
            spawn(async move {
                sleep_ms(TOAST_TTL_MS).await;
                begin_dismiss_toast(&mut toasts, id);
            });
        }
    }
}

/// Предоставляет клиенту глобальные toast-уведомления.
#[component]
pub(crate) fn ToastProvider(children: Element) -> Element {
    let toasts = use_signal(Vec::<Toast>::new);
    let next_id = use_signal(|| 0_u64);
    let handle = ToastHandle { toasts, next_id };
    use_context_provider(move || handle);

    rsx! {
        {children}
        div {
            class: "pointer-events-none fixed inset-x-0 top-3 z-[1100] flex flex-col items-center gap-2 px-3 sm:inset-x-auto sm:right-4 sm:top-4 sm:w-[420px] sm:items-stretch sm:px-0",
            for toast in toasts() {
                {render_toast(toast, toasts)}
            }
        }
    }
}

fn render_toast(toast: Toast, toasts: Signal<Vec<Toast>>) -> Element {
    match toast.payload.clone() {
        ToastPayload::Message(message) => render_message_toast(toast, message, toasts),
        ToastPayload::UpdateAvailable(update) => {
            render_update_available_toast(toast, update, toasts)
        }
    }
}

fn render_message_toast(toast: Toast, message: String, mut toasts: Signal<Vec<Toast>>) -> Element {
    rsx! {
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
                p { class: "break-words text-[13px] leading-5 text-zinc-300", "{message}" }
            }
            button {
                r#type: "button",
                "aria-label": "Закрыть уведомление",
                class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[18px] leading-none text-zinc-500 transition hover:bg-white/5 hover:text-zinc-100",
                onclick: move |_| begin_dismiss_toast(&mut toasts, toast.id),
                "×"
            }
            if !toast.kind.persistent() {
                span { class: "toast-progress absolute bottom-0 left-0 h-px {toast.kind.accent_class()}" }
            }
        }
    }
}

fn render_update_available_toast(
    toast: Toast,
    update: UpdateAvailableToast,
    mut toasts: Signal<Vec<Toast>>,
) -> Element {
    let on_install = update.on_install.clone();
    let on_quick_dismiss = update.on_quick_dismiss.clone();
    let on_defer = update.on_defer.clone();
    let selected_deferral_value = update.selected_deferral_value.clone();

    rsx! {
        article {
            key: "{toast.id}",
            role: toast.kind.role(),
            "aria-live": toast.kind.live_region(),
            class: update_toast_class(toast.exiting),
            div { class: "flex items-start gap-3 px-3 py-3",
                div { class: "mt-1 flex h-5 w-5 shrink-0 items-center justify-center",
                    span { class: "h-2.5 w-2.5 rounded-full {toast.kind.accent_class()} shadow-[0_0_18px_rgba(96,165,250,0.55)]" }
                }
                div { class: "min-w-0 flex-1 space-y-1",
                    p { class: "text-[12px] font-semibold leading-4 text-zinc-100", "{toast.kind.label()}" }
                    p { class: "break-words text-[13px] leading-5 text-zinc-300",
                        "CheenHub {update.current_version} → {update.update_version}"
                    }
                    p { class: "text-[12px] leading-5 text-zinc-500",
                        if let Some(title) = update.title.as_ref() {
                            "{title}"
                        } else {
                            "На GitHub опубликован новый релиз."
                        }
                    }
                }
                button {
                    r#type: "button",
                    "aria-label": "Скрыть уведомление об обновлении на пять минут",
                    class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[18px] leading-none text-zinc-500 transition hover:bg-white/5 hover:text-zinc-100",
                    onclick: move |_| {
                        (on_quick_dismiss.as_ref())();
                        begin_dismiss_toast(&mut toasts, toast.id);
                    },
                    "×"
                }
            }
            div { class: "grid gap-2 border-t border-white/10 bg-white/[0.025] px-3 py-3",
                button {
                    r#type: "button",
                    disabled: update.primary_disabled,
                    class: update_primary_button_class(update.primary_disabled),
                    onclick: move |_| (on_install.as_ref())(),
                    "{update.primary_label}"
                }
                div { class: "grid grid-cols-[1fr_auto] gap-2",
                    select {
                        value: "{update.selected_deferral_value}",
                        class: "h-9 min-w-0 rounded-md border border-zinc-800 bg-zinc-950 px-2.5 text-[12px] font-medium text-zinc-200 outline-none transition focus:border-blue-400/60 focus:ring-4 focus:ring-blue-400/10",
                        onchange: move |event| set_update_deferral_value(&mut toasts, toast.id, event.value()),
                        for option in update.deferral_options.iter() {
                            option {
                                value: "{option.value}",
                                selected: option.value == update.selected_deferral_value,
                                "{option.label}"
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "flex h-9 items-center justify-center rounded-md border border-zinc-800 bg-zinc-900/80 px-3 text-[12px] font-semibold text-zinc-200 transition hover:border-zinc-700 hover:bg-zinc-900",
                        onclick: move |_| {
                            (on_defer.as_ref())(selected_deferral_value.clone());
                            begin_dismiss_toast(&mut toasts, toast.id);
                        },
                        "Позже"
                    }
                }
            }
            span { class: "block h-px w-full bg-blue-400" }
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

fn update_toast_class(exiting: bool) -> &'static str {
    if exiting {
        "toast-item toast-item-exiting pointer-events-auto w-full max-w-[calc(100vw-1.5rem)] overflow-hidden rounded-lg border border-white/10 bg-zinc-950/95 text-zinc-100 shadow-[0_18px_50px_rgba(0,0,0,0.38)] backdrop-blur sm:max-w-none"
    } else {
        "toast-item pointer-events-auto w-full max-w-[calc(100vw-1.5rem)] overflow-hidden rounded-lg border border-white/10 bg-zinc-950/95 text-zinc-100 shadow-[0_18px_50px_rgba(0,0,0,0.38)] backdrop-blur sm:max-w-none"
    }
}

fn update_primary_button_class(disabled: bool) -> &'static str {
    if disabled {
        "flex h-9 cursor-not-allowed items-center justify-center rounded-md border border-zinc-800 bg-zinc-900/70 px-3 text-[12px] font-semibold text-zinc-500"
    } else {
        "flex h-9 items-center justify-center rounded-md border border-blue-400/25 bg-blue-500/10 px-3 text-[12px] font-semibold text-blue-100 transition hover:border-blue-400/40 hover:bg-blue-500/15"
    }
}

fn set_update_deferral_value(toasts: &mut Signal<Vec<Toast>>, id: u64, value: String) {
    let mut next_toasts = toasts();
    let Some(toast) = next_toasts.iter_mut().find(|toast| toast.id == id) else {
        return;
    };
    let ToastPayload::UpdateAvailable(update) = &mut toast.payload else {
        return;
    };

    update.selected_deferral_value = value;
    toasts.set(next_toasts);
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
        sleep_ms(TOAST_EXIT_MS).await;
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
