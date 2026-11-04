//! Проверка настоящего дерева Dioxus и интерактивных спойлеров.

mod support;

use super::MarkdownContent;
use dioxus::dioxus_core::{DynamicNode, ElementId, MountedVNode, Mutation};
use dioxus::prelude::*;
use std::cell::RefCell;
use support::click;

thread_local! {
    static BODY: RefCell<Option<Signal<String>>> = const { RefCell::new(None) };
    static COPIED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct TestDocument {
    owner: dioxus::dioxus_core::internal::generational_box::Owner,
}

struct TestEvaluator;

impl dioxus::document::Evaluator for TestEvaluator {
    fn send(&self, data: serde_json::Value) -> Result<(), dioxus::document::EvalError> {
        COPIED.with_borrow_mut(|copied| copied.push(data.as_str().unwrap().to_owned()));
        Ok(())
    }
    fn poll_recv(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, dioxus::document::EvalError>> {
        std::task::Poll::Pending
    }
    fn poll_join(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, dioxus::document::EvalError>> {
        std::task::Poll::Ready(Ok(serde_json::Value::Bool(true)))
    }
}

impl dioxus::document::Document for TestDocument {
    fn eval(&self, _: String) -> dioxus::document::Eval {
        let evaluator: Box<dyn dioxus::document::Evaluator> = Box::new(TestEvaluator);
        dioxus::document::Eval::new(self.owner.insert(evaluator))
    }
}

fn copy_code_app() -> Element {
    let document: std::rc::Rc<dyn dioxus::document::Document> = std::rc::Rc::new(TestDocument {
        owner: Default::default(),
    });
    use_context_provider(|| document);
    rsx! { MarkdownContent { body: "```rust\n  let x = 1;\n```".to_owned() } }
}

fn message_app() -> Element {
    let body = use_signal(|| "||[секрет](https://example.com)|| и ||второй||".to_owned());
    BODY.with_borrow_mut(|slot| *slot = Some(body));
    rsx! { MarkdownContent { body: body() } }
}

fn formatting_app() -> Element {
    rsx! { MarkdownContent { body: "**жирный *курсив*** __подчёркнутый__ ~~нет~~ `код`\n\n```rust\n  пробел\nстрока\n```\n\n# заголовок\n\n## второй\n\n### третий\n\n> цитата\n\n- список\n\n3. нумерация\n\n| а | б |\n| :- | -: |\n| 1 | 2 |\n\n<script>alert(1)</script> 😀".to_owned() } }
}

fn links_app() -> Element {
    rsx! { MarkdownContent { body: "[safe](https://example.com) [bad](javascript:alert) [file](file:///etc/passwd)".to_owned() } }
}

fn chat_item_app() -> Element {
    use crate::features::{
        app::current_user::CurrentUserContext, application_focus::ApplicationFocusContext,
        text_chat::ChatMessageItem, toast::ToastProvider,
    };
    use cheenhub_contracts::{realtime::TextChatMessage, rest::AuthUser};
    let user = use_signal(|| {
        Some(AuthUser {
            id: "user".into(),
            nickname: "Имя".into(),
            email: "a@example.com".into(),
            registered_at: "2026-01-01T00:00:00Z".into(),
            has_password: true,
            avatar_url: None,
        })
    });
    let focus = use_signal(|| true);
    use_context_provider(|| CurrentUserContext::new(user));
    use_context_provider(|| ApplicationFocusContext::new(focus));
    let body = use_signal(|| "Старое **сообщение**".to_owned());
    BODY.with_borrow_mut(|slot| *slot = Some(body));
    let message = TextChatMessage {
        id: "message".into(),
        server_id: "server".into(),
        room_id: "room".into(),
        author_user_id: "user".into(),
        author_nickname: "Имя".into(),
        author_avatar_url: None,
        body: body(),
        attachments: vec![],
        delivery_status: None,
        created_at: "2026-01-01T00:00:00Z".into(),
        edited_at: None,
    };
    rsx! { ToastProvider { ChatMessageItem { message, animate: false, removing: false, can_delete_messages: false, on_delete: move |_| {}, } } }
}

fn node_text(dom: &VirtualDom, node: MountedVNode<'_>, output: &mut String) {
    for (op, _) in node.template().decoded_ops().enumerate() {
        if let Some(text) = node.template().static_text(op) {
            output.push_str(text.text());
        }
    }
    for slot in node.dynamic_anchors().flat_map(|anchor| anchor.nodes()) {
        match &*slot {
            DynamicNode::Text(text) => output.push_str(&text.value),
            DynamicNode::Fragment(_) => {
                for child in node.mounted_fragment_children(slot, dom) {
                    node_text(dom, child, output);
                }
            }
            DynamicNode::Component(component) => {
                if let Some(child) = component
                    .mounted_scope(slot, node, dom)
                    .and_then(|scope| scope.try_mounted_root_node())
                {
                    node_text(dom, child, output);
                }
            }
        }
    }
}

fn text(dom: &VirtualDom) -> String {
    let mut output = String::new();
    node_text(
        dom,
        dom.base_scope().try_mounted_root_node().unwrap(),
        &mut output,
    );
    output
}

fn clicks(edits: &[Mutation]) -> Vec<ElementId> {
    let mut current = None;
    edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::SetId { id } | Mutation::PushId { id } => {
                current = Some(*id);
                None
            }
            Mutation::NewEventListener { name } if name == "click" => current,
            _ => None,
        })
        .collect()
}

#[test]
fn spoilers_reveal_independently_and_hidden_link_is_not_mounted() {
    let mut dom = VirtualDom::new(message_app);
    let initial = dom.rebuild_to_vec();
    let buttons = clicks(&initial.edits);
    assert_eq!(buttons.len(), 2);
    assert!(!text(&dom).contains("Спойлер"));
    assert!(!format!("{:?}", initial.edits).contains("tag: \"button\""));
    assert!(!text(&dom).contains("секрет"));
    assert!(!format!("{:?}", initial.edits).contains("https://example.com"));

    click(&dom, buttons[0]);
    let revealed = dom.render_immediate_to_vec();
    assert!(text(&dom).contains("секрет"));
    assert!(!text(&dom).contains("второй"));
    assert!(format!("{:?}", revealed.edits).contains("https://example.com"));

    click(&dom, buttons[0]);
    dom.render_immediate_to_vec();
    assert!(!text(&dom).contains("секрет"));
    click(&dom, buttons[1]);
    dom.render_immediate_to_vec();
    assert!(text(&dom).contains("второй"));
    assert!(!text(&dom).contains("секрет"));
}

#[test]
fn markdown_rendering_keeps_text_code_unicode_and_html_inert() {
    let mut dom = VirtualDom::new(formatting_app);
    let mutations = dom.rebuild_to_vec();
    let output = text(&dom);
    for value in [
        "жирный",
        "курсив",
        "подчёркнутый",
        "  пробел\nстрока\n",
        "заголовок",
        "цитата",
        "список",
        "<script>alert(1)</script>",
        "😀",
    ] {
        assert!(output.contains(value), "{value:?} absent from {output:?}");
    }
    let output = format!("{:?}", mutations.edits);
    for value in [
        "strong",
        "em",
        "tag: \"u\"",
        "tag: \"s\"",
        "tag: \"code\"",
        "blockquote",
        "pre",
        "tag: \"ul\"",
        "tag: \"ol\"",
        "text-2xl",
        "text-xl",
        "text-lg",
        "table",
        "overflow-x-auto",
    ] {
        assert!(output.contains(value), "{value} absent from {output}");
    }
    assert!(!output.contains("tag: \"script\""));
}

#[test]
fn edited_message_replaces_old_content_and_resets_spoiler() {
    let mut dom = VirtualDom::new(message_app);
    let initial = dom.rebuild_to_vec();
    click(&dom, clicks(&initial.edits)[0]);
    dom.render_immediate_to_vec();
    assert!(text(&dom).contains("секрет"));

    BODY.with_borrow_mut(|body| body.as_mut().unwrap().set("Изменено **теперь**".to_owned()));
    dom.render_immediate_to_vec();
    assert!(text(&dom).contains("теперь"));
    assert!(!text(&dom).contains("секрет"));
}

#[test]
fn unchanged_message_rerender_keeps_spoiler_revealed() {
    let mut dom = VirtualDom::new(message_app);
    let initial = dom.rebuild_to_vec();
    click(&dom, clicks(&initial.edits)[0]);
    dom.render_immediate_to_vec();
    assert!(text(&dom).contains("секрет"));

    BODY.with_borrow_mut(|body| {
        body.as_mut()
            .unwrap()
            .set("||[секрет](https://example.com)|| и ||второй||".into())
    });
    dom.render_immediate_to_vec();
    assert!(text(&dom).contains("секрет"));
}

#[test]
fn unsafe_link_schemes_never_create_navigation_targets() {
    let mut dom = VirtualDom::new(links_app);
    let mutations = dom.rebuild_to_vec();
    let hrefs: Vec<_> = mutations
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::SetAttribute { name, value, .. } if name == "href" => {
                Some(format!("{value:?}"))
            }
            _ => None,
        })
        .collect();
    assert_eq!(hrefs.len(), 1, "{hrefs:?}");
    assert!(hrefs[0].contains("https://example.com"));
}

#[test]
fn pinned_preview_never_contains_spoiler_contents() {
    let result = super::markdown_preview("До ||**секрет**|| после", None);
    assert!(result.contains("До "));
    assert!(result.contains("Спойлер"));
    assert!(result.contains(" после"));
    assert!(!result.contains("секрет"));
}

#[test]
fn rejected_input_does_not_reveal_spoiler_in_preview() {
    let input = format!("||секрет||{}", "a".repeat(70_000));
    assert!(!super::markdown_preview(&input, None).contains("секрет"));
}

#[test]
fn chat_message_item_renders_old_message_as_markdown() {
    let mut dom = VirtualDom::new(chat_item_app);
    let initial = dom.rebuild_to_vec();
    assert!(text(&dom).contains("сообщение"));
    assert!(!text(&dom).contains("**"));
    assert!(format!("{:?}", initial.edits).contains("strong"));
}

#[test]
fn copying_code_block_sends_original_content_without_markers() {
    COPIED.with_borrow_mut(Vec::clear);
    let mut dom = VirtualDom::new(copy_code_app);
    let initial = dom.rebuild_to_vec();
    let buttons = clicks(&initial.edits);

    click(&dom, buttons[0]);
    dom.render_immediate_to_vec();

    COPIED.with_borrow(|copied| assert_eq!(copied.as_slice(), &["  let x = 1;\n"]));
}
