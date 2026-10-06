//! Регрессии загрузки и сохранения системных настроек в Dioxus runtime.

use super::*;
use cheenhub_contracts::rest::HostLogLevel;
use dioxus::dioxus_core::{AttributeValue, Mutation};
use std::cell::RefCell;

thread_local! {
    static LOADED: RefCell<Option<Signal<Option<HostLogSettingsResponse>>>> = const { RefCell::new(None) };
}

#[derive(Clone, PartialEq, Routable)]
enum TestRoute {
    #[route("/")]
    Settings {},
}

fn delayed_settings_app() -> Element {
    rsx! { Router::<TestRoute> {} }
}

#[component]
fn Settings() -> Element {
    let loaded = use_signal(|| None::<HostLogSettingsResponse>);
    LOADED.with_borrow_mut(|slot| *slot = Some(loaded));
    settings_content(loaded().map(Ok), EventHandler::new(|_| {}))
}

#[test]
fn loaded_level_is_selected_after_initial_loading_render() {
    let mut dom = VirtualDom::new(delayed_settings_app);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let mut loaded = LOADED.with_borrow(|slot| slot.unwrap());
        loaded.set(Some(HostLogSettingsResponse {
            min_level: Some(HostLogLevel::Debug),
            updated_at: None,
        }));
    });

    let mutations = dom.render_immediate_to_vec();
    let startup_disabled = mutations
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "disabled",
                value: AttributeValue::Bool(value),
                ..
            } => Some(*value),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        startup_disabled,
        vec![false, true],
        "loaded debug must enable startup reset and disable unchanged save"
    );
}
