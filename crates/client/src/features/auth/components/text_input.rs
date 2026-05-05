//! Text input component for the authentication form.

use dioxus::prelude::*;

#[component]
pub(crate) fn TextInput(
    input_type: &'static str,
    label: &'static str,
    name: &'static str,
    placeholder: &'static str,
    autocomplete: &'static str,
    value: String,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block",
            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "{label}" }
            input {
                r#type: input_type,
                name,
                placeholder,
                autocomplete,
                value,
                oninput: move |event| oninput.call(event.value()),
                class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[14px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
            }
        }
    }
}
