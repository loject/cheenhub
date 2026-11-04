use super::can_save_edit;
mod support;

#[test]
fn inline_editor_offers_markdown_preview() {
    use super::RoomMessageInlineEditor;
    use cheenhub_contracts::realtime::TextChatMessage;
    use dioxus::dioxus_core::{DynamicNode, Mutation};
    use dioxus::prelude::*;

    fn app() -> Element {
        let message = TextChatMessage {
            id: "message".into(),
            server_id: "server".into(),
            room_id: "room".into(),
            author_user_id: "user".into(),
            author_nickname: "Имя".into(),
            author_avatar_url: None,
            body: "Привет **мир** 😀".into(),
            attachments: vec![],
            delivery_status: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            edited_at: None,
        };
        rsx! { RoomMessageInlineEditor { message, saving: false, error: None, on_cancel: move |_| {}, on_save: move |_| {} } }
    }
    fn collect(dom: &VirtualDom, node: dioxus::dioxus_core::MountedVNode<'_>, out: &mut String) {
        for (op, _) in node.template().decoded_ops().enumerate() {
            if let Some(text) = node.template().static_text(op) {
                out.push_str(text.text());
            }
        }
        for slot in node.dynamic_anchors().flat_map(|anchor| anchor.nodes()) {
            match &*slot {
                DynamicNode::Text(text) => out.push_str(&text.value),
                DynamicNode::Fragment(_) => {
                    for child in node.mounted_fragment_children(slot, dom) {
                        collect(dom, child, out);
                    }
                }
                DynamicNode::Component(component) => {
                    if let Some(child) = component
                        .mounted_scope(slot, node, dom)
                        .and_then(|scope| scope.try_mounted_root_node())
                    {
                        collect(dom, child, out);
                    }
                }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    let mut output = String::new();
    collect(
        &dom,
        dom.base_scope().try_mounted_root_node().unwrap(),
        &mut output,
    );
    assert!(output.contains("Предпросмотр"));
    let mut current = None;
    let button = initial
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::SetId { id } | Mutation::PushId { id } => {
                current = Some(*id);
                None
            }
            Mutation::NewEventListener { name } if name == "click" => current,
            _ => None,
        })
        .expect("preview button click listener");

    support::click(&dom, button);
    let opened = dom.render_immediate_to_vec();
    output.clear();
    collect(
        &dom,
        dom.base_scope().try_mounted_root_node().unwrap(),
        &mut output,
    );
    assert!(output.contains("Редактировать"));
    assert!(output.contains("мир"));
    for edit in &opened.edits {
        if let Mutation::SetAttribute { name, value, .. } = edit
            && name == "value"
        {
            assert!(format!("{value:?}").contains("Привет **мир** 😀"));
        }
    }

    support::click(&dom, button);
    let closed = dom.render_immediate_to_vec();
    output.clear();
    collect(
        &dom,
        dom.base_scope().try_mounted_root_node().unwrap(),
        &mut output,
    );
    assert!(output.contains("Предпросмотр"));
    assert!(
        closed.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name, value, .. }
                if name == "style" && format!("{value:?}").contains("display: block")
        )),
        "textarea must explicitly become visible after preview"
    );
    for edit in &closed.edits {
        if let Mutation::SetAttribute { name, value, .. } = edit
            && name == "value"
        {
            assert!(format!("{value:?}").contains("Привет **мир** 😀"));
        }
    }
}

#[test]
fn empty_text_requires_existing_attachment() {
    assert!(!can_save_edit("   ", "Текст", false, 2000));
    assert!(can_save_edit("   ", "Текст", true, 2000));
}

#[test]
fn unchanged_or_too_long_text_is_not_saved() {
    assert!(!can_save_edit(" Текст ", "Текст", false, 2000));
    assert!(!can_save_edit(&"я".repeat(2001), "Текст", false, 2000));
    assert!(can_save_edit("Исправленный текст", "Текст", false, 2000));
}

#[test]
fn dm_editor_allows_4000_characters_and_clearing_caption_with_image() {
    assert!(can_save_edit(&"я".repeat(4000), "Текст", false, 4000));
    assert!(!can_save_edit(&"я".repeat(4001), "Текст", false, 4000));
    assert!(can_save_edit("", "Подпись", true, 4000));
    assert!(!can_save_edit("", "Подпись", false, 4000));
}
