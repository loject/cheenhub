//! User profile settings section.

use dioxus::prelude::*;

use super::styles::{input_class, primary_button_class};

const MOCK_NICKNAME: &str = "chingiz";
const MOCK_EMAIL: &str = "chingiz@example.com";

/// Renders mock profile and account controls.
#[component]
pub(crate) fn ProfileSettingsSection() -> Element {
    let mut nickname = use_signal(|| MOCK_NICKNAME.to_owned());
    let mut email = use_signal(|| MOCK_EMAIL.to_owned());
    let mut email_password = use_signal(String::new);
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut repeat_new_password = use_signal(String::new);
    let mut avatar_selected = use_signal(|| false);
    let profile_changed = nickname() != MOCK_NICKNAME || email() != MOCK_EMAIL || avatar_selected();
    let password_changed = !current_password().is_empty()
        || !new_password().is_empty()
        || !repeat_new_password().is_empty();
    let has_changes = profile_changed || !email_password().is_empty() || password_changed;

    rsx! {
        form { class: "space-y-4",
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-col gap-4 sm:flex-row sm:items-center",
                    div { class: "flex h-20 w-20 shrink-0 items-center justify-center rounded-2xl bg-accent text-[28px] font-bold text-white shadow-[0_14px_36px_rgba(59,130,246,.20)]", "Ч" }
                    div { class: "min-w-0 flex-1",
                        h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Аватар" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Загрузи изображение, которое будет отображаться в профиле и списках участников." }
                        label { class: "mt-3 inline-flex h-9 cursor-pointer items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-3 text-[12px] font-medium text-zinc-200 transition hover:border-zinc-700 hover:bg-zinc-900",
                            input {
                                class: "sr-only",
                                r#type: "file",
                                accept: "image/*",
                                onchange: move |_| avatar_selected.set(true),
                            }
                            "Выбрать изображение"
                        }
                    }
                }
            }
            div { class: "grid gap-4 lg:grid-cols-2",
                div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                    h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Данные профиля" }
                    div { class: "mt-4 space-y-3",
                        label { class: "block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Никнейм" }
                            input {
                                r#type: "text",
                                value: nickname(),
                                maxlength: "32",
                                autocomplete: "nickname",
                                oninput: move |event| nickname.set(event.value()),
                                class: input_class(),
                            }
                        }
                        label { class: "block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Email" }
                            input {
                                r#type: "email",
                                value: email(),
                                autocomplete: "email",
                                oninput: move |event| email.set(event.value()),
                                class: input_class(),
                            }
                        }
                        label { class: "block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Пароль для смены email" }
                            input {
                                r#type: "password",
                                value: email_password(),
                                autocomplete: "current-password",
                                oninput: move |event| email_password.set(event.value()),
                                class: input_class(),
                            }
                        }
                    }
                }
                div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                    h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Пароль" }
                    div { class: "mt-4 space-y-3",
                        label { class: "block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Текущий пароль" }
                            input {
                                r#type: "password",
                                value: current_password(),
                                autocomplete: "current-password",
                                oninput: move |event| current_password.set(event.value()),
                                class: input_class(),
                            }
                        }
                        label { class: "block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Новый пароль" }
                            input {
                                r#type: "password",
                                value: new_password(),
                                autocomplete: "new-password",
                                oninput: move |event| new_password.set(event.value()),
                                class: input_class(),
                            }
                        }
                        label { class: "block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Повторите новый пароль" }
                            input {
                                r#type: "password",
                                value: repeat_new_password(),
                                autocomplete: "new-password",
                                oninput: move |event| repeat_new_password.set(event.value()),
                                class: input_class(),
                            }
                        }
                    }
                }
            }
            div { class: "flex justify-end",
                button {
                    r#type: "button",
                    disabled: !has_changes,
                    class: primary_button_class(),
                    "Сохранить профиль"
                }
            }
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Связанные аккаунты" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Подключай внешние аккаунты для входа и будущих интеграций." }
                div { class: "mt-4 space-y-2",
                    div { class: "flex flex-col gap-3 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-3 sm:flex-row sm:items-center sm:justify-between",
                        div { class: "min-w-0 flex items-center gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-[13px] font-semibold text-zinc-100", "G" }
                            div { class: "min-w-0",
                                p { class: "truncate text-[13px] font-medium text-zinc-100", "Google" }
                                p { class: "mt-0.5 truncate text-[11px] text-zinc-500", "chingiz@gmail.com" }
                            }
                        }
                        button {
                            r#type: "button",
                            class: "flex h-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-red-500/35 hover:bg-red-500/10 hover:text-red-200",
                            "Отключить"
                        }
                    }
                    div { class: "group relative flex flex-col gap-3 rounded-2xl border border-zinc-800 bg-zinc-900/35 p-3 opacity-60 sm:flex-row sm:items-center sm:justify-between",
                        div { class: "min-w-0 flex items-center gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-[13px] font-semibold text-blue-200", "D" }
                            div { class: "min-w-0",
                                p { class: "truncate text-[13px] font-medium text-zinc-100", "Discord" }
                                p { class: "mt-0.5 truncate text-[11px] text-zinc-500", "Подключение аккаунта" }
                            }
                        }
                        button {
                            r#type: "button",
                            disabled: true,
                            class: "flex h-9 shrink-0 cursor-not-allowed items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-500",
                            "Подключить"
                        }
                        span { class: "pointer-events-none absolute left-1/2 top-full z-10 mt-2 w-max -translate-x-1/2 rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-2 text-[12px] text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-opacity duration-200 group-hover:opacity-100",
                            "в разработке"
                        }
                    }
                }
            }
        }
    }
}
