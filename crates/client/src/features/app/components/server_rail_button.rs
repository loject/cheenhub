//! Server rail button component.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

/// Renders a server switcher button.
#[component]
pub(crate) fn ServerRailButton(
    server: ServerSummary,
    active: bool,
    on_select_server: EventHandler<String>,
) -> Element {
    let button_class = if active {
        "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px relative flex h-12 w-12 items-center justify-center rounded-2xl border border-accent/40 bg-accent text-sm font-semibold text-white"
    } else {
        "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-sm font-semibold text-zinc-300 hover:border-zinc-700 hover:bg-zinc-800"
    };
    let label = initials(&server.name);
    let server_id = server.id.clone();

    rsx! {
        button {
            r#type: "button",
            class: button_class,
            "aria-label": "{server.name}",
            onclick: move |_| on_select_server.call(server_id.clone()),
            if active {
                span { class: "absolute -left-3 h-7 w-1 rounded-r-full bg-accent" }
            }
            "{label}"
        }
    }
}

fn initials(name: &str) -> String {
    let mut initials = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    if initials.is_empty() {
        initials = name.chars().take(2).collect::<String>().to_uppercase();
    }

    initials
}
