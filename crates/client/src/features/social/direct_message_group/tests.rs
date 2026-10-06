use cheenhub_contracts::rest::{DmImageAttachmentSummary, DmMessageSummary};

use super::prepare_direct_message_groups;

fn message(id: &str, sender: &str, created_at: &str) -> DmMessageSummary {
    DmMessageSummary {
        id: id.to_owned(),
        conversation_id: "conversation".to_owned(),
        seq: id.parse().unwrap_or_default(),
        sender_user_id: sender.to_owned(),
        sender_nickname: sender.to_owned(),
        sender_avatar_url: None,
        body: "текст".to_owned(),
        image: None,
        delivery_status: None,
        created_at: created_at.to_owned(),
    }
}

#[test]
fn virtual_dm_groups_preserve_order_and_split_on_author_or_day() {
    let messages = [
        message("1", "alice", "2025-07-12T08:00:00Z"),
        message("2", "alice", "2025-07-12T09:00:00Z"),
        message("3", "bob", "2025-07-12T10:00:00Z"),
        message("4", "bob", "2025-07-13T10:00:00Z"),
    ];

    let groups = prepare_direct_message_groups(&messages);

    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].0, "1");
    assert_eq!(
        groups[0]
            .3
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["1", "2"]
    );
    assert_eq!(groups[1].0, "3");
    assert_eq!(groups[2].0, "4");
    assert!(groups[0].1.is_some());
    assert!(groups[1].1.is_none());
    assert!(groups[2].1.is_some());
}

#[test]
fn virtual_dm_group_estimate_accounts_for_an_image() {
    let plain = prepare_direct_message_groups(&[message("1", "alice", "2025-07-12T08:00:00Z")]);
    let mut rich_message = message("2", "alice", "2025-07-12T08:00:00Z");
    rich_message.image = Some(DmImageAttachmentSummary {
        id: "image".to_owned(),
        content_type: "image/png".to_owned(),
        width: 1_600,
        height: 900,
    });
    let rich = prepare_direct_message_groups(&[rich_message]);

    assert!(rich[0].2 > plain[0].2 + 290.0);
}
