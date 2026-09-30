//! Web-реализация вставки изображения из Dioxus paste-события.

use dioxus::html::{FileData, HasFileData};
use dioxus::prelude::*;

use crate::features::message_composer::pending_attachment::{
    PendingImageAttachment, pending_image_attachment,
};

use crate::features::message_composer::MAX_IMAGE_BYTES;

/// Синхронно извлекает файл через Dioxus и запускает чтение в Dioxus runtime.
pub(crate) fn read_pasted_image(
    event: ClipboardEvent,
    on_outcome: EventHandler<Result<PendingImageAttachment, String>>,
) -> bool {
    let files = event.files();
    let file_count = files.len();
    let Some(file) = files.into_iter().find(|file| {
        file.content_type()
            .as_deref()
            .is_some_and(is_supported_image_mime)
    }) else {
        debug!(
            file_count,
            "message composer browser paste has no supported image item"
        );
        return false;
    };

    event.prevent_default();
    let file_name = (!file.name().trim().is_empty()).then(|| file.name());
    let byte_size = file.size();
    info!(
        has_file_name = file_name.is_some(),
        byte_size, "found message composer image in browser paste"
    );
    spawn(async move {
        let result = read_image_file(file, file_name).await;
        match &result {
            Ok(attachment) => info!(
                byte_size = attachment.byte_size,
                "added pending message composer image from browser paste"
            ),
            Err(error) => warn!(%error, "rejected message composer image from browser paste"),
        }
        on_outcome.call(result);
    });
    true
}

/// Web не читает системный буфер по keydown: файл доступен только в paste-событии.
pub(crate) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    Ok(None)
}

pub(crate) fn supports_keydown_image_paste() -> bool {
    false
}

async fn read_image_file(
    file: FileData,
    file_name: Option<String>,
) -> Result<PendingImageAttachment, String> {
    let buffer = file
        .read_bytes()
        .await
        .map_err(|_| "Не удалось прочитать изображение из буфера обмена.".to_owned())?;
    pending_image_attachment(file_name, buffer.to_vec(), MAX_IMAGE_BYTES)
}

fn is_supported_image_mime(value: &str) -> bool {
    matches!(
        value,
        "image/jpeg" | "image/png" | "image/webp" | "image/gif"
    )
}
