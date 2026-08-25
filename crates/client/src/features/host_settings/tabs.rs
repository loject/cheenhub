//! Навигация между разделами настроек хоста.

use dioxus::prelude::*;

use crate::Route;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HostSettingsTab {
    Dashboard,
    Email,
    Logs,
}

pub(super) fn host_settings_tabs(active: HostSettingsTab) -> Element {
    rsx! {
        nav {
            class: "mt-5 grid max-w-[560px] grid-cols-3 gap-1 rounded-xl border border-zinc-800 bg-zinc-950/70 p-1",
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
            Link {
                to: Route::AppHostLogs {},
                class: tab_class(active == HostSettingsTab::Logs),
                "Логи"
            }
        }
    }
}

fn tab_class(active: bool) -> &'static str {
    if active {
        "flex min-h-10 items-center justify-center rounded-lg border border-accent/25 bg-accent/10 px-4 text-[12px] font-medium text-blue-100 transition-[background-color,border-color,color,transform] duration-150 active:scale-[0.97]"
    } else {
        "flex min-h-10 items-center justify-center rounded-lg border border-transparent px-4 text-[12px] font-medium text-zinc-400 transition-[background-color,border-color,color,transform] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 active:scale-[0.97]"
    }
}
