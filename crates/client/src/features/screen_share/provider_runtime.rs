//! Вспомогательные функции runtime-провайдера демонстрации экрана.

use dioxus::prelude::*;

use super::backend::{ScreenShareError, ScreenShareStatus};

pub(super) fn status_from_error(error: ScreenShareError) -> ScreenShareStatus {
    if error.is_permission_denied() {
        ScreenShareStatus::PermissionDenied
    } else {
        ScreenShareStatus::Error(error.to_string())
    }
}

pub(super) fn next_generation(generation: &mut Signal<u64>) -> u64 {
    let next_generation = generation.peek().saturating_add(1);
    generation.set(next_generation);
    next_generation
}
