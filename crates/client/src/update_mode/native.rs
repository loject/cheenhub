//! Native-точка выбора updater-режима.

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#[path = "../updater/mod.rs"]
mod updater;

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
pub(super) fn run_if_requested() -> bool {
    const UPDATE_MODE_ARG: &str = "--cheenhub-update";

    if !std::env::args().any(|arg| arg == UPDATE_MODE_ARG) {
        return false;
    }

    updater::run();
    true
}

#[cfg(not(all(feature = "desktop", not(target_arch = "wasm32"))))]
pub(super) fn run_if_requested() -> bool {
    false
}
