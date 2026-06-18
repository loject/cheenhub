//! Вспомогательные функции runtime-провайдера камеры.

use dioxus::prelude::*;

use super::backend::{CameraError, CameraStatus};

pub(super) fn status_from_error(error: CameraError) -> CameraStatus {
    if error.is_permission_denied() {
        CameraStatus::PermissionDenied
    } else {
        CameraStatus::Error(error.to_string())
    }
}

pub(super) fn next_generation(generation: &mut Signal<u64>) -> u64 {
    let next_generation = generation.peek().saturating_add(1);
    generation.set(next_generation);
    next_generation
}
