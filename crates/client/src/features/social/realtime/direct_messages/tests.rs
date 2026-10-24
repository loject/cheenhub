//! Проверки пользовательских ошибок и ограничения размера realtime-вложений ЛС.

use super::*;
use dioxus::prelude::*;
use futures_util::FutureExt;

#[test]
fn disconnected_history_returns_actionable_message_without_transport_details() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let realtime = crate::features::realtime::create_handle();

        let result = list_dm_messages(&realtime, "conversation", None)
            .now_or_never()
            .expect("disconnected request completes");

        assert_eq!(
            result.unwrap_err(),
            "Не удалось загрузить сообщения. Проверь соединение и повтори попытку."
        );
    });
}

#[test]
fn oversized_image_is_rejected_before_network_request() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let realtime = crate::features::realtime::create_handle();

        let result = upload_dm_image(&realtime, "conversation", vec![0; 8 * 1024 * 1024 + 1])
            .now_or_never()
            .expect("oversized upload completes");

        assert_eq!(
            result.unwrap_err(),
            "Изображение слишком большое. Максимум — 8 МБ."
        );
    });
}

#[test]
fn server_rejection_preserves_product_message() {
    let error = RealtimeError::rejected("Писать можно только друзьям.");

    assert_eq!(
        user_error(&error, "Не удалось отправить сообщение."),
        "Писать можно только друзьям."
    );
}
