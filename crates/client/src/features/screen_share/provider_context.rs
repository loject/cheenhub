//! Компонент провайдера контекста демонстрации экрана.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::core::current_scope_id;
use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::backend::{ScreenShareSession, ScreenShareStatus};
use super::native::default_backend;
use super::picker_host::ScreenSharePickerHost;
use super::provider::ScreenShareHandle;

/// Предоставляет состояние захвата экрана аутентифицированным компонентам приложения.
#[component]
pub(crate) fn ScreenShareProvider(children: Element) -> Element {
    let status = use_signal(|| ScreenShareStatus::Idle);
    let session = use_signal(|| None::<Rc<dyn ScreenShareSession>>);
    let generation = use_signal(|| 0);
    let picker_open = use_signal(|| false);
    let toast = use_context::<ToastHandle>();
    let backend = default_backend();
    let handle = ScreenShareHandle {
        provider_scope: current_scope_id(),
        status,
        session,
        generation,
        backend,
        toast,
        picker_open,
        pending_on_frame: Rc::new(RefCell::new(None)),
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
        if picker_open() {
            ScreenSharePickerHost { open: picker_open }
        }
    }
}
