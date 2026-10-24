//! Проверки имён команд ЛС и принадлежности ответов модулю Social.

use super::*;

#[test]
fn direct_message_commands_and_responses_round_trip_in_social_module() {
    let request_id = Uuid::new_v4();
    for kind in [
        "list_direct_message_conversations",
        "direct_message_conversations",
        "open_direct_message_conversation",
        "direct_message_conversation_opened",
        "load_direct_message_history",
        "direct_message_history",
        "send_direct_message",
        "send_direct_message_accepted",
        "edit_direct_message",
        "edit_direct_message_accepted",
        "mark_direct_message_read",
        "mark_direct_message_read_accepted",
        "upload_direct_message_image",
        "upload_direct_message_image_accepted",
        "load_direct_message_image",
        "direct_message_image_loaded",
    ] {
        let wire = serde_json::json!({
            "module": "social", "kind": kind, "request_id": request_id, "payload": {}
        });

        let envelope: RealtimeEnvelope =
            serde_json::from_value(wire.clone()).unwrap_or_else(|error| panic!("{kind}: {error}"));

        assert!(envelope.has_matching_module_kind(), "{kind}");
        assert_eq!(serde_json::to_value(envelope).unwrap(), wire, "{kind}");
    }
}
