//! Компонент провайдера контекста демонстрации экрана.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::backend::{ScreenShareSession, ScreenShareStatus};
use super::native::default_backend;
use super::provider::ScreenShareHandle;

/// Предоставляет состояние захвата экрана аутентифицированным компонентам приложения.
#[component]
pub(crate) fn ScreenShareProvider(children: Element) -> Element {
    let status = use_signal(|| ScreenShareStatus::Idle);
    let session = use_signal(|| None::<Rc<dyn ScreenShareSession>>);
    let generation = use_signal(|| 0);
    let toast = use_context::<ToastHandle>();
    let backend = default_backend();
    let handle = ScreenShareHandle {
        status,
        session,
        generation,
        backend,
        toast,
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
    }
}
