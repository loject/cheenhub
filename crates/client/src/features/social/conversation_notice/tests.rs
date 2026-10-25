//! Обновление предупреждения и доступного действия после изменения дружбы.

use super::*;
use dioxus::dioxus_core::{DynamicNode, TemplateNode, VNode};
use futures_util::FutureExt;

fn app() -> Element {
    let relation = use_context::<Signal<Option<UserRelationStatus>>>();
    rsx! {
        ConversationNotice {
            user_id: "peer".to_owned(),
            relation: relation(),
            incoming_request_id: Some("request".to_owned()),
            on_changed: move |_| {},
        }
    }
}

fn pump(dom: &mut VirtualDom) {
    for _ in 0..5 {
        let _ = dom.wait_for_work().now_or_never();
        dom.render_immediate_to_vec();
    }
}

fn text(dom: &VirtualDom) -> String {
    fn static_text(nodes: &[TemplateNode], output: &mut String) {
        for node in nodes {
            match node {
                TemplateNode::Text { text } => output.push_str(text),
                TemplateNode::Element { children, .. } => static_text(children, output),
                _ => {}
            }
        }
    }
    fn visit(dom: &VirtualDom, node: &VNode, output: &mut String) {
        static_text(node.template.roots(), output);
        for (index, dynamic) in node.dynamic_nodes.iter().enumerate() {
            match dynamic {
                DynamicNode::Text(text) => output.push_str(&text.value),
                DynamicNode::Fragment(children) => {
                    for child in children {
                        visit(dom, child, output);
                    }
                }
                DynamicNode::Component(component) => {
                    if let Some(scope) = component.mounted_scope(index, node, dom) {
                        visit(dom, scope.root_node(), output);
                    }
                }
                _ => {}
            }
        }
    }
    let mut output = String::new();
    visit(dom, dom.base_scope().root_node(), &mut output);
    output
}

#[test]
fn friendship_updates_hide_notice_and_removal_restores_it() {
    let mut dom = VirtualDom::new(app);
    let mut relation = dom.in_scope(ScopeId::ROOT, || Signal::new(None));
    dom.provide_root_context(relation);
    dom.rebuild_in_place();
    pump(&mut dom);
    assert!(text(&dom).contains("Добавить в друзья"));
    assert!(text(&dom).contains("Блокировка в разработке"));

    dom.in_scope(ScopeId::ROOT, || {
        relation.set(Some(UserRelationStatus::PendingOutgoing))
    });
    pump(&mut dom);
    assert!(text(&dom).contains("Заявка отправлена"));

    dom.in_scope(ScopeId::ROOT, || {
        relation.set(Some(UserRelationStatus::Friends))
    });
    pump(&mut dom);
    assert!(text(&dom).is_empty());

    dom.in_scope(ScopeId::ROOT, || relation.set(None));
    pump(&mut dom);
    assert!(text(&dom).contains("Добавить в друзья"));
}

#[test]
fn incoming_request_offers_acceptance_instead_of_duplicate_request() {
    let mut dom = VirtualDom::new(app);
    let relation = dom.in_scope(ScopeId::ROOT, || {
        Signal::new(Some(UserRelationStatus::PendingIncoming))
    });
    dom.provide_root_context(relation);

    dom.rebuild_in_place();
    pump(&mut dom);

    assert!(text(&dom).contains("Принять заявку"));
    assert!(!text(&dom).contains("Добавить в друзья"));
}
