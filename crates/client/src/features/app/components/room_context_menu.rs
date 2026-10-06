//! Контекстное меню управления комнатами вне фильтрованной панели.

use super::room_menu_policy::RoomMenuCommand;
use dioxus::prelude::*;
use std::rc::Rc;

/// Показывает разрешённые команды возле указателя и поддерживает клавиатуру.
///
/// Координаты ограничены размером окна; вызывающий scope проверяет актуальные права
/// и существование комнаты перед открытием и выполнением команды.
#[component]
pub(super) fn RoomContextMenu(
    commands: Vec<RoomMenuCommand>,
    label: String,
    x: f64,
    y: f64,
    on_command: EventHandler<RoomMenuCommand>,
    on_close: EventHandler<()>,
) -> Element {
    let mut anchors = use_signal(Vec::<(usize, Rc<MountedData>)>::new);
    let count = commands.len();
    let position = format!(
        "left: clamp(8px, {x}px, calc(100vw - 248px)); top: clamp(8px, {y}px, calc(100dvh - 180px));"
    );
    rsx! {
        button {
            r#type: "button", tabindex: "-1",
            class: "fixed inset-0 z-[999] cursor-default",
            "aria-label": "Закрыть меню комнат",
            onclick: move |_| on_close.call(()),
            oncontextmenu: move |event| { event.prevent_default(); on_close.call(()); },
        }
        div {
            role: "menu", "aria-label": "Управление комнатами",
            class: "fixed z-[1000] w-[240px] max-w-[calc(100vw-16px)] rounded-2xl border border-zinc-800 bg-zinc-950 p-1.5 shadow-2xl",
            style: position,
            onclick: move |event| event.stop_propagation(),
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    event.stop_propagation();
                    on_close.call(());
                }
            },
            p { class: "truncate px-3 py-2 text-xs font-semibold text-zinc-500", "{label}" }
            for (index, command) in commands.into_iter().enumerate() {
                button {
                    key: "{index}", r#type: "button", role: "menuitem",
                    class: if command == RoomMenuCommand::Delete {
                        "flex min-h-10 w-full items-center rounded-xl px-3 text-left text-sm text-red-200 hover:bg-red-500/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-red-400/70"
                    } else {
                        "flex min-h-10 w-full items-center rounded-xl px-3 text-left text-sm text-zinc-200 hover:bg-zinc-800 focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
                    },
                    onmounted: move |event| {
                        let element = event.data();
                        anchors.write().push((index, element.clone()));
                        if index == 0 {
                            spawn(async move {
                                if let Err(error) = element.set_focus(true).await {
                                    debug!(%error, "could not focus room menu");
                                }
                            });
                        }
                    },
                    onkeydown: move |event| {
                        let next = match event.key() {
                            Key::ArrowDown => (index + 1) % count,
                            Key::ArrowUp => (index + count - 1) % count,
                            Key::Home => 0,
                            Key::End => count - 1,
                            Key::Tab => { on_close.call(()); return; }
                            _ => return,
                        };
                        event.prevent_default();
                        let element = anchors.read().iter().find(|(i, _)| *i == next).map(|(_, e)| e.clone());
                        if let Some(element) = element {
                            spawn(async move {
                                if let Err(error) = element.set_focus(true).await {
                                    debug!(%error, "could not move room menu focus");
                                }
                            });
                        }
                    },
                    onclick: move |_| on_command.call(command),
                    match command {
                        RoomMenuCommand::Create => "Создать комнату",
                        RoomMenuCommand::Edit => "Редактировать комнату",
                        RoomMenuCommand::Delete => "Удалить комнату",
                    }
                }
            }
        }
    }
}
