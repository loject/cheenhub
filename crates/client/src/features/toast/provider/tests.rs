//! Проверки обновления постоянного toast без потери пользовательского состояния.

use super::*;
use crate::features::application_focus::ApplicationFocusContext;
use crate::features::toast::{
    UpdateAvailableToastActions, UpdateAvailableToastContent, UpdateToastDeferralOption,
};

thread_local! {
    static HANDLE: std::cell::Cell<Option<ToastHandle>> = const { std::cell::Cell::new(None) };
}

fn app() -> Element {
    let focused = use_signal(|| true);
    use_context_provider(|| ApplicationFocusContext::new(focused));
    rsx! { Probe {} }
}

#[component]
fn Probe() -> Element {
    let toasts = use_signal(Vec::new);
    let next_id = use_signal(|| 0);
    let application_focus = use_context();
    HANDLE.set(Some(ToastHandle {
        toasts,
        next_id,
        application_focus,
    }));
    rsx! {}
}

fn update(label: &str) -> UpdateAvailableToast {
    UpdateAvailableToast::new(
        UpdateAvailableToastContent::new(
            "1",
            "2",
            None,
            label,
            false,
            vec![UpdateToastDeferralOption::new("tomorrow", "Завтра")],
            "tomorrow",
        ),
        UpdateAvailableToastActions::new(|| {}, || {}, |_| {}),
    )
}

#[test]
fn refresh_preserves_notification_identity_and_selected_deferral() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let mut handle = HANDLE.get().unwrap();
        handle.update_available(update("Скачать"));
        let id = handle.toasts.peek()[0].id;
        super::update_view::set_update_deferral_value(&mut handle.toasts, id, "week".into());

        handle.update_available(update("Установить"));

        let toasts = handle.toasts.peek();
        assert_eq!(toasts.len(), 1);
        assert_eq!(toasts[0].id, id);
        let ToastPayload::UpdateAvailable(content) = &toasts[0].payload else {
            panic!()
        };
        assert_eq!(content.primary_label, "Установить");
        assert_eq!(content.selected_deferral_value, "week");
    });
}

#[test]
fn ordinary_messages_do_not_evict_update_notification() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let handle = HANDLE.get().unwrap();
        handle.update_available(update("Скачать"));

        for _ in 0..MAX_TOASTS {
            handle.info("Сообщение");
        }

        assert!(
            handle
                .toasts
                .peek()
                .iter()
                .any(|toast| toast.kind == ToastKind::UpdateAvailable)
        );
    });
}

#[test]
fn repeated_custom_event_keeps_its_identity() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let handle = HANDLE.get().unwrap();
        handle.custom("incoming:call-1", || rsx! { div { "Первый" } });
        let first_id = handle.toasts.peek()[0].id;

        handle.custom("incoming:call-1", || rsx! { div { "Повтор" } });

        assert_eq!(handle.toasts.peek().len(), 1);
        assert_eq!(handle.toasts.peek()[0].id, first_id);
    });
}

#[test]
fn custom_event_remains_visible_without_countdown_or_eviction() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let handle = HANDLE.get().unwrap();
        handle.custom("incoming:call-1", || rsx! { div { "Звонок" } });

        assert!(!handle.toasts.peek()[0].countdown_active());
        for _ in 0..MAX_TOASTS {
            handle.info("Обычное сообщение");
        }

        assert!(handle.toasts.peek().iter().any(|toast| {
            matches!(&toast.payload, ToastPayload::Custom(custom) if custom.key == "incoming:call-1")
        }));
    });
}

#[test]
fn dismiss_custom_event_only_closes_matching_key() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let handle = HANDLE.get().unwrap();
        handle.custom("incoming:call-1", || rsx! { div { "Первый" } });
        handle.custom("incoming:call-2", || rsx! { div { "Второй" } });

        handle.dismiss_custom("incoming:call-1");

        let toasts = handle.toasts.peek();
        assert!(toasts[0].exiting);
        assert!(!toasts[1].exiting);
    });
}
