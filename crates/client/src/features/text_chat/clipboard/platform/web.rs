//! Web-реализация чтения изображения из Dioxus paste-события.

use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use super::super::super::pending_attachment::{
    PendingImageAttachment, is_supported_image_mime, pending_image_attachment,
};

/// Синхронно извлекает файл из browser event, пока `DataTransfer` доступен.
pub(crate) fn read_pasted_image(
    event: ClipboardEvent,
    on_outcome: EventHandler<Result<PendingImageAttachment, String>>,
) -> bool {
    let Some(browser_event) = browser_clipboard_event(&event) else {
        warn!("text chat paste event did not contain a browser ClipboardEvent");
        return false;
    };
    let Some(data) = browser_event.clipboard_data() else {
        debug!("text chat browser paste has no clipboard data");
        return false;
    };
    let item_count = data.items().length();
    let file_count = data.files().as_ref().map_or(0, web_sys::FileList::length);
    let Some(file) = find_image_file(&data) else {
        debug!(
            item_count,
            file_count, "text chat browser paste has no supported image item"
        );
        return false;
    };

    browser_event.prevent_default();
    let file_name = (!file.name().trim().is_empty()).then(|| file.name());
    let byte_size = file.size();
    info!(
        has_file_name = file_name.is_some(),
        byte_size, "found text chat image in browser paste"
    );
    spawn(async move {
        let result = read_image_file(file, file_name).await;
        match &result {
            Ok(attachment) => info!(
                byte_size = attachment.byte_size,
                has_preview = attachment.preview_data_url.is_some(),
                "accepted text chat image from browser paste"
            ),
            Err(error) => warn!(%error, "rejected text chat image from browser paste"),
        }
        on_outcome.call(result);
    });
    true
}

fn browser_clipboard_event(event: &ClipboardEvent) -> Option<web_sys::ClipboardEvent> {
    let browser_event = event.data().downcast::<web_sys::Event>()?.clone();
    browser_event.dyn_into::<web_sys::ClipboardEvent>().ok()
}

async fn read_image_file(
    file: web_sys::File,
    file_name: Option<String>,
) -> Result<PendingImageAttachment, String> {
    let buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|_| "Не удалось прочитать изображение из буфера обмена.".to_owned())?;
    pending_image_attachment(
        file_name,
        js_sys::Uint8Array::new(&buffer).to_vec(),
        10 * 1024 * 1024,
    )
}

fn find_image_file(data: &web_sys::DataTransfer) -> Option<web_sys::File> {
    data.files()
        .and_then(|files| {
            (0..files.length())
                .filter_map(|index| files.get(index))
                .find(|file| is_supported_image_mime(&file.type_()))
        })
        .or_else(|| {
            (0..data.items().length()).find_map(|index| {
                let item = data.items().get(index)?;
                (item.kind() == "file" && is_supported_image_mime(&item.type_()))
                    .then(|| item.get_as_file())
                    .and_then(|result| result.ok().flatten())
            })
        })
}
