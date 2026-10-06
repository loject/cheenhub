//! Проверки вложений личных сообщений.

use super::dm_image_kind;
use uuid::Uuid;

#[test]
fn image_kind_is_scoped_to_one_direct_conversation() {
    let first_conversation_id = Uuid::new_v4();
    let second_conversation_id = Uuid::new_v4();

    assert_ne!(
        dm_image_kind(first_conversation_id),
        dm_image_kind(second_conversation_id)
    );
    assert!(dm_image_kind(first_conversation_id).ends_with(&first_conversation_id.to_string()));
}
