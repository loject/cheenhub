//! Навигация между разделами настроек хоста.

use dioxus::prelude::*;

use crate::Route;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HostSettingsTab {
    Dashboard,
    Email,
}

pub(super) fn host_settings_tabs(active: HostSettingsTab) -> Element {
    rsx! {
        nav {
            class: "mt-6 grid max-w-lg grid-cols-2 rounded-2xl bg-zinc-900/75 p-1.5 shadow-[0_0_0_1px_rgba(255,255,255,0.07)]",
            "aria-label": "Разделы настроек хоста",
            Link {
                to: Route::AppHostSettings {},
                class: tab_class(active == HostSettingsTab::Dashboard),
                "Дашборд"
            }
            Link {
                to: Route::AppHostEmailSettings { gmail: None, email: None },
                class: tab_class(active == HostSettingsTab::Email),
                "Исходящая почта"
            }
        }
    }
}

fn tab_class(active: bool) -> &'static str {
    if active {
        "flex min-h-11 items-center justify-center rounded-xl bg-blue-500 px-4 text-[13px] font-semibold text-white shadow-[0_8px_24px_rgba(59,130,246,0.18)] transition-[background-color,color,scale] duration-150 active:scale-[0.96]"
    } else {
        "flex min-h-11 items-center justify-center rounded-xl px-4 text-[13px] font-semibold text-zinc-400 transition-[background-color,color,scale] duration-150 hover:bg-zinc-800 hover:text-white active:scale-[0.96]"
    }
}
