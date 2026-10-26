//! Регрессии загрузки владельцев и состояния строк в Dioxus runtime.

use super::*;
use dioxus::dioxus_core::{DynamicNode, ElementId, MountedVNode, Mutation};
use std::cell::RefCell;

mod support;
use support::click;

thread_local! {
    static RESOURCE: RefCell<Option<Resource<Result<HostOwnersResponse, api::HostSettingsApiError>>>> = const { RefCell::new(None) };
    static RESPONSE: RefCell<Result<HostOwnersResponse, api::HostSettingsApiError>> = RefCell::new(Err(api::HostSettingsApiError::Other("offline".into())));
    static PENDING: RefCell<Option<Signal<Vec<String>>>> = const { RefCell::new(None) };
    static OWNERS: RefCell<Option<Signal<Vec<HostOwnerSummary>>>> = const { RefCell::new(None) };
}

fn owner(id: &str) -> HostOwnerSummary {
    HostOwnerSummary {
        user_id: id.into(),
        nickname: id.into(),
        email: format!("{id}@example.com"),
        avatar_url: None,
        granted_at: "2026-10-05T10:00:00Z".into(),
        granted_by_nickname: None,
        is_current_user: false,
    }
}

fn load_app() -> Element {
    let resource = use_resource(|| async { RESPONSE.with_borrow(Clone::clone) });
    RESOURCE.with_borrow_mut(|slot| *slot = Some(resource));
    let (owners, error) = use_owners_load(resource);
    OWNERS.with_borrow_mut(|slot| *slot = Some(owners));
    rsx! { p { "{owners().len()}:{error():?}" } }
}

fn rows_app() -> Element {
    let owners = use_signal(|| vec![owner("alice"), owner("bob"), owner("carol")]);
    OWNERS.with_borrow_mut(|slot| *slot = Some(owners));
    let pending = use_signal(Vec::<String>::new);
    PENDING.with_borrow_mut(|slot| *slot = Some(pending));
    owner_list(&owners(), &pending(), EventHandler::new(|_| {}))
}

fn pump(dom: &mut VirtualDom) {
    for _ in 0..5 {
        dom.render_immediate_to_vec();
    }
}

fn node_text(dom: &VirtualDom, node: MountedVNode<'_>, out: &mut String) {
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
                    node_text(dom, child, out);
                }
            }
            DynamicNode::Component(component) => {
                if let Some(child) = component
                    .mounted_scope(slot, node, dom)
                    .and_then(|scope| scope.try_mounted_root_node())
                {
                    node_text(dom, child, out);
                }
            }
        }
    }
}

fn text(dom: &VirtualDom) -> String {
    let mut out = String::new();
    node_text(
        dom,
        dom.base_scope().try_mounted_root_node().unwrap(),
        &mut out,
    );
    out
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
fn retry_after_load_failure_applies_new_owner_response() {
    RESPONSE.with_borrow_mut(|response| {
        *response = Err(api::HostSettingsApiError::Other("offline".into()))
    });
    let mut dom = VirtualDom::new(load_app);
    dom.rebuild_in_place();
    pump(&mut dom);
    assert!(text(&dom).contains("offline"));
    RESPONSE.with_borrow_mut(|response| {
        *response = Ok(HostOwnersResponse {
            owners: vec![owner("alice")],
        })
    });

    dom.in_scope(ScopeId::ROOT, || {
        RESOURCE.with_borrow(|slot| slot.unwrap().restart())
    });
    pump(&mut dom);

    assert_eq!(text(&dom), "1:None");
}

#[test]
fn failed_revoke_allows_confirmation_again() {
    let mut dom = VirtualDom::new(rows_app);
    let initial = dom.rebuild_to_vec();
    click(&dom, clicks(&initial.edits)[0]);
    let confirmation = dom.render_immediate_to_vec();

    click(&dom, clicks(&confirmation.edits)[0]);
    pump(&mut dom);

    assert!(
        text(&dom).contains("Да, отозвать"),
        "failed operation must allow another attempt"
    );
    assert!(!text(&dom).contains("Отзываем..."));
}

#[test]
fn removing_owner_does_not_transfer_confirmation_to_next_owner() {
    let mut dom = VirtualDom::new(rows_app);
    let initial = dom.rebuild_to_vec();
    click(&dom, clicks(&initial.edits)[0]);
    pump(&mut dom);
    assert!(text(&dom).contains("Отозвать права?"));

    dom.in_scope(ScopeId::ROOT, || {
        OWNERS.with_borrow(|slot| slot.unwrap().write().remove(0))
    });
    pump(&mut dom);

    assert!(
        !text(&dom).contains("Отозвать права?"),
        "bob must retain his own unconfirmed row"
    );
}

fn loading_app() -> Element {
    owners_body(
        OwnersView {
            loading: true,
            revoking_owners: vec![],
            owners: vec![],
            load_error: Some("offline".into()),
            revoke_error: None,
            revoke_notice: None,
        },
        EventHandler::new(|_| {}),
        EventHandler::new(|_| {}),
        EventHandler::new(|_| {}),
    )
}

#[test]
fn loading_owner_list_replaces_previous_error_with_loader() {
    let mut dom = VirtualDom::new(loading_app);
    dom.rebuild_in_place();

    assert!(text(&dom).contains("Загружаем владельцев"));
    assert!(!text(&dom).contains("offline"));
}

#[test]
fn refresh_after_success_replaces_stale_owner_list() {
    RESPONSE.with_borrow_mut(|response| {
        *response = Ok(HostOwnersResponse {
            owners: vec![owner("alice"), owner("bob")],
        })
    });
    let mut dom = VirtualDom::new(load_app);
    dom.rebuild_in_place();
    pump(&mut dom);
    assert_eq!(text(&dom), "2:None");
    RESPONSE.with_borrow_mut(|response| {
        *response = Ok(HostOwnersResponse {
            owners: vec![owner("alice")],
        })
    });

    dom.in_scope(ScopeId::ROOT, || {
        RESOURCE.with_borrow(|slot| slot.unwrap().restart())
    });
    pump(&mut dom);

    assert_eq!(text(&dom), "1:None");
}

fn refreshing_app() -> Element {
    owners_body(
        OwnersView {
            loading: true,
            revoking_owners: vec![],
            owners: vec![owner("alice")],
            load_error: None,
            revoke_error: None,
            revoke_notice: None,
        },
        EventHandler::new(|_| {}),
        EventHandler::new(|_| {}),
        EventHandler::new(|_| {}),
    )
}

#[test]
fn refresh_keeps_loaded_list_and_grant_form_visible() {
    let mut dom = VirtualDom::new(refreshing_app);
    dom.rebuild_in_place();

    assert!(text(&dom).contains("Загружаем владельцев"));
    assert!(text(&dom).contains("Выдать права владельца"));
    assert!(text(&dom).contains("alice@example.com"));
}

#[test]
fn local_owner_update_is_not_overwritten_by_previous_load() {
    RESPONSE.with_borrow_mut(|response| {
        *response = Ok(HostOwnersResponse {
            owners: vec![owner("alice")],
        })
    });
    let mut dom = VirtualDom::new(load_app);
    dom.rebuild_in_place();
    pump(&mut dom);
    assert_eq!(text(&dom), "1:None");

    dom.in_scope(ScopeId::ROOT, || {
        OWNERS.with_borrow(|slot| slot.unwrap().write().push(owner("bob")))
    });
    pump(&mut dom);

    assert_eq!(text(&dom), "2:None");
}

#[test]
fn completed_revoke_enables_confirmation_controls() {
    let mut dom = VirtualDom::new(rows_app);
    let initial = dom.rebuild_to_vec();
    click(&dom, clicks(&initial.edits)[0]);
    pump(&mut dom);
    dom.in_scope(ScopeId::ROOT, || {
        PENDING.with_borrow(|slot| slot.unwrap().set(vec!["alice".into()]))
    });
    pump(&mut dom);
    assert!(text(&dom).contains("Отзываем..."));

    dom.in_scope(ScopeId::ROOT, || {
        PENDING.with_borrow(|slot| slot.unwrap().set(vec![]))
    });
    let completion = dom.render_immediate_to_vec();

    assert!(text(&dom).contains("Да, отозвать"));
    let enabled_controls = completion.edits.iter().filter(|edit| matches!(edit,
        Mutation::SetAttribute { name, value: dioxus::dioxus_core::AttributeValue::Bool(false), .. } if *name == "disabled"
    )).count();
    assert_eq!(
        enabled_controls, 2,
        "both retry and cancel must be enabled after completion"
    );
}
