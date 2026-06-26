//! Компонент провайдера контекста камеры.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::backend::{CameraSession, CameraStatus};
use super::native::default_backend;
use super::provider::CameraHandle;

/// Предоставляет состояние камеры аутентифицированным компонентам приложения.
#[component]
pub(crate) fn CameraProvider(children: Element) -> Element {
    let status = use_signal(|| CameraStatus::Idle);
    let session = use_signal(|| None::<Rc<dyn CameraSession>>);
    let generation = use_signal(|| 0);
    let toast = use_context::<ToastHandle>();
    let backend = default_backend();
    let handle = CameraHandle {
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
