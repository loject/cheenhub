//! Проверки обновления постоянного toast без потери пользовательского состояния.

use super::*;
use crate::features::application_focus::ApplicationFocusProvider;
use crate::features::toast::{
    UpdateAvailableToastActions, UpdateAvailableToastContent, UpdateToastDeferralOption,
};

thread_local! {
    static HANDLE: std::cell::Cell<Option<ToastHandle>> = const { std::cell::Cell::new(None) };
}

fn app() -> Element {
    rsx! { ApplicationFocusProvider { Probe {} } }
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
