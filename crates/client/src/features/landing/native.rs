//! Выбор платформенной реализации лендинга.

#[cfg(not(target_family = "wasm"))]
pub(crate) use super::desktop::{
    LandingRoute, public_home_label, public_home_route, public_landing_available,
};
#[cfg(target_family = "wasm")]
pub(crate) use super::web::{
    LandingRoute, public_home_label, public_home_route, public_landing_available,
};
