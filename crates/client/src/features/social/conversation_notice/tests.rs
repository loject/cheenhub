//! Обновление предупреждения и доступного действия после изменения дружбы.

use super::*;
use dioxus::dioxus_core::{DynamicNode, MountedVNode};
use futures_util::FutureExt;

fn app() -> Element {
    let relation = use_context::<Signal<Option<UserRelationStatus>>>();
    rsx! {
        ConversationNotice {
            user_id: "peer".to_owned(),
            relation: relation(),
            block_status: DmBlockStatus::None,
            incoming_request_id: Some("request".to_owned()),
            on_changed: move |_| {},
        }
    }
}

fn blocked_app() -> Element {
    let block_status = use_context::<Signal<DmBlockStatus>>();
    rsx! {
        ConversationNotice {
            user_id: "peer".to_owned(),
            relation: None,
            block_status: block_status(),
            incoming_request_id: None,
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
    fn visit(dom: &VirtualDom, node: MountedVNode<'_>, output: &mut String) {
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
                        visit(dom, child, output);
                    }
                }
                DynamicNode::Component(component) => {
                    if let Some(child) = component
                        .mounted_scope(slot, node, dom)
                        .and_then(|scope| scope.try_mounted_root_node())
                    {
                        visit(dom, child, output);
                    }
                }
            }
        }
    }
    let mut output = String::new();
    visit(
        dom,
        dom.base_scope().try_mounted_root_node().unwrap(),
        &mut output,
    );
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
    assert!(!text(&dom).contains("Блокировка в разработке"));

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
fn blocked_notices_hide_friendship_and_offer_unblock_only_to_blocker() {
    let mut dom = VirtualDom::new(blocked_app);
    let mut block_status = dom.in_scope(ScopeId::ROOT, || Signal::new(DmBlockStatus::BlockedByMe));
    dom.provide_root_context(block_status);
    dom.rebuild_in_place();
    pump(&mut dom);
    let own_block = text(&dom);
    assert!(own_block.contains("Вы заблокировали этого пользователя"));
    assert!(own_block.contains("Разблокировать"));
    assert!(!own_block.contains("Добавить в друзья"));

    dom.in_scope(ScopeId::ROOT, || {
        block_status.set(DmBlockStatus::BlockedByPeer)
    });
    pump(&mut dom);
    let peer_block = text(&dom);
    assert!(peer_block.contains("Пользователь вас заблокировал"));
    assert!(!peer_block.contains("Разблокировать"));
    assert!(!peer_block.contains("Добавить в друзья"));

    dom.in_scope(ScopeId::ROOT, || block_status.set(DmBlockStatus::Mutual));
    pump(&mut dom);
    let mutual = text(&dom);
    assert!(mutual.contains("Вы заблокировали друг друга"));
    assert!(mutual.contains("Разблокировать"));

    dom.in_scope(ScopeId::ROOT, || block_status.set(DmBlockStatus::None));
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
