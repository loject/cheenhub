//! Общий нижний блок боковой панели приложения.

use dioxus::prelude::*;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::user_settings::UserSettingsScope;
use crate::features::voice_chat::SidebarVoiceControls;

use super::avatar::UserAvatar;
use super::profile_menu::ProfileMenu;
use super::server_realtime_status::ServerRealtimeStatus;
use super::server_rooms_sidebar_styles as sidebar_styles;

/// Рендерит общий блок realtime-статуса, голосовых контролов и профиля пользователя.
#[component]
pub(crate) fn AppSidebarFooter(
    realtime_label: String,
    settings_workspace_active: bool,
    show_voice_controls: bool,
) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let mut is_user_settings_open = use_signal(|| false);
    let mut is_profile_menu_open = use_signal(|| false);
    let sidebar_voice_class = sidebar_styles::sidebar_voice_class(settings_workspace_active);
    let user_bar_class = sidebar_styles::user_bar_class(settings_workspace_active);
    let user_details_class = sidebar_styles::user_details_class(settings_workspace_active);

    rsx! {
        div {
            class: "relative z-40 border-t border-zinc-800/80 p-3",
            onclick: move |_| is_profile_menu_open.set(false),
            ServerRealtimeStatus {
                label: realtime_label,
                settings_workspace_active,
            }
            if show_voice_controls {
                div { class: sidebar_voice_class,
                    SidebarVoiceControls {}
                }
            }
            div { class: user_bar_class,
                button {
                    r#type: "button",
                    class: "flex min-w-0 flex-1 items-center gap-3 rounded-[14px] text-left transition-[background,color] duration-150 hover:bg-zinc-800/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/50",
                    "aria-label": "Меню профиля",
                    "aria-expanded": if is_profile_menu_open() { "true" } else { "false" },
                    onclick: move |event| {
                        event.stop_propagation();
                        is_profile_menu_open.set(!is_profile_menu_open());
                    },
                    UserAvatar {
                        nickname: current_user.nickname.clone(),
                        avatar_url: current_user.avatar_url.clone(),
                        class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-accent text-[12px] font-bold text-white".to_owned(),
                        avatar_seed: Some(current_user.id.clone()),
                    }
                    div { class: user_details_class,
                        div { class: "truncate text-[12px] font-medium text-zinc-100", "{current_user.nickname}" }
                        div { class: "truncate text-[11px] text-zinc-500", "в приложении" }
                    }
                }
                button {
                    r#type: "button",
                    class: "rounded-lg p-2 text-zinc-500 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-zinc-800 hover:text-zinc-200",
                    "aria-label": "Настройки пользователя",
                    onclick: move |event| {
                        event.stop_propagation();
                        is_profile_menu_open.set(false);
                        is_user_settings_open.set(true);
                    },
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.075.04.149.083.22.127.324.2.72.226 1.075.091l1.217-.462a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.82c-.293.24-.438.613-.431.992a7.723 7.723 0 0 1 0 .255c-.007.379.138.752.431.992l1.003.82c.433.354.54.972.26 1.431l-1.296 2.247a1.125 1.125 0 0 1-1.37.49l-1.217-.462c-.355-.135-.751-.109-1.076.091a6.72 6.72 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.542-.56.94-1.11.94h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.063-.374-.313-.686-.645-.87a6.52 6.52 0 0 1-.22-.127c-.324-.2-.72-.226-1.075-.091l-1.217.462a1.125 1.125 0 0 1-1.37-.49l-1.296-2.247a1.125 1.125 0 0 1 .26-1.431l1.003-.82c.293-.24.438-.613.431-.992a6.932 6.932 0 0 1 0-.255c.007-.379-.138-.752-.431-.992l-1.003-.82a1.125 1.125 0 0 1-.26-1.431l1.296-2.247a1.125 1.125 0 0 1 1.37-.49l1.217.462c.355-.135.751-.109 1.076-.091.071-.044.145-.087.220-.128.331-.183.581-.495.644-.869l.213-1.281Z" }
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" }
                    }
                }
            }
            if is_profile_menu_open() {
                ProfileMenu { on_close: move |_| is_profile_menu_open.set(false) }
            }
        }
        if is_user_settings_open() {
            UserSettingsScope { on_close: move |_| is_user_settings_open.set(false) }
        }
    }
}
