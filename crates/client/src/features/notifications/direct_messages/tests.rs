use super::requires_attention;

#[test]
fn focused_open_conversation_does_not_require_attention() {
    assert!(!requires_attention(true, true));
}

#[test]
fn background_or_other_conversation_requires_attention() {
    assert!(requires_attention(false, true));
    assert!(requires_attention(true, false));
    assert!(requires_attention(false, false));
}
