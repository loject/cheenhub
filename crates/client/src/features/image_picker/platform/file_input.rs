//! Выбор изображения через стандартный file input вне Android.

use dioxus::prelude::*;

use super::super::backend::{ImagePickerOutcome, PickedImage, oversized_image_message};

/// Показывает кнопку выбора одного изображения через платформенный file input.
#[component]
pub(crate) fn ImagePickerButton(
    disabled: bool,
    busy: bool,
    max_bytes: usize,
    on_outcome: EventHandler<ImagePickerOutcome>,
    on_active_change: EventHandler<bool>,
) -> Element {
    let mut is_reading = use_signal(|| false);
    let unavailable = disabled || busy || is_reading();
    let select_image = use_callback(move |event: Event<FormData>| {
        if disabled || busy || is_reading() {
            return;
        }
        let Some(file) = event.files().into_iter().next() else {
            return;
        };
        if file.size() > max_bytes as u64 {
            on_outcome.call(ImagePickerOutcome::Failed(oversized_image_message(
                max_bytes,
            )));
            return;
        }

        let file_name = (!file.name().trim().is_empty()).then(|| file.name());
        let file_size = file.size();
        is_reading.set(true);
        on_active_change.call(true);
        info!(file_size, "reading selected message image");
        spawn(async move {
            match file.read_bytes().await {
                Ok(bytes) if !bytes.is_empty() => {
                    on_outcome.call(ImagePickerOutcome::Selected(PickedImage {
                        file_name,
                        bytes: bytes.to_vec(),
                    }));
                }
                Ok(_) => {
                    warn!("selected message image is empty");
                    on_outcome.call(ImagePickerOutcome::Failed(
                        "Выбранное изображение пустое.".to_owned(),
                    ));
                }
                Err(error) => {
                    warn!(?error, "failed to read selected message image");
                    on_outcome.call(ImagePickerOutcome::Failed(
                        "Не удалось прочитать выбранное изображение.".to_owned(),
                    ));
                }
            }
            is_reading.set(false);
            on_active_change.call(false);
        });
    });

    rsx! {
        label {
            class: "flex size-10 shrink-0 cursor-pointer items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-300 transition-[background-color,border-color,color,transform,opacity] duration-150 ease-out hover:-translate-y-px hover:border-white/15 hover:bg-zinc-800 hover:text-zinc-100 active:scale-[0.96] has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-45 has-[:disabled]:hover:translate-y-0 has-[:disabled]:active:scale-100",
            title: "Прикрепить изображение",
            input {
                class: "sr-only",
                r#type: "file",
                name: "message-image",
                accept: "image/png,image/jpeg,image/gif,image/webp",
                disabled: unavailable,
                "aria-label": "Прикрепить изображение",
                onchange: move |event| select_image.call(event),
            }
            if busy || is_reading() {
                span { class: "size-4 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300", "aria-hidden": "true" }
            } else {
                svg { class: "size-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "m18.375 12.739-7.693 7.693a4.5 4.5 0 0 1-6.364-6.364l10.94-10.94a3 3 0 1 1 4.243 4.243L8.552 18.32a1.5 1.5 0 1 1-2.121-2.121l9.879-9.879" }
                }
            }
        }
    }
}
