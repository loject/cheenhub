//! User context menu component.

use dioxus::prelude::*;

/// Renders a fixed-position user context menu.
#[component]
pub(crate) fn UserContextMenu(name: &'static str, volume: &'static str, x: f64, y: f64) -> Element {
    let top = y + 8.0;
    let style = format!(
        "left: clamp(12px, {x}px, calc(100vw - 258px)); top: clamp(12px, {top}px, calc(100vh - 260px));"
    );

    rsx! {
        div {
            class: "user-menu fixed z-[1000] w-[246px] rounded-[20px] border border-zinc-800 bg-zinc-950/95 p-2 shadow-[0_22px_70px_rgba(0,0,0,.60)] backdrop-blur-xl",
            style,
            onclick: move |event| event.stop_propagation(),
            div { class: "px-2 pb-2 pt-1",
                div { class: "text-[12px] font-medium text-zinc-200", "{name}" }
                div { class: "text-[11px] text-zinc-500", "индивидуальная громкость" }
                input { class: "mt-3 w-full accent-blue-500", r#type: "range", min: "0", max: "200", value: volume }
                div { class: "mt-1 flex justify-between text-[10px] text-zinc-600",
                    span { "0%" }
                    span { "{volume}%" }
                    span { "200%" }
                }
            }
            div { class: "my-1 border-t border-zinc-800" }
            button { r#type: "button", class: "flex w-full items-center justify-between rounded-xl px-3 py-2.5 text-left text-[13px] text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:bg-zinc-900 hover:text-zinc-100",
                span { "Кикнуть из голоса" }
                span { class: "text-[10px] text-zinc-600", "админ" }
            }
            button { r#type: "button", class: "flex w-full items-center justify-between rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:bg-red-500/10 hover:text-red-200",
                span { "Кикнуть с сервера" }
                span { class: "text-[10px] text-red-400/60", "админ" }
            }
        }
    }
}
