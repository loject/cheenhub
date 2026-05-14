//! Image processing and public delivery feature.

pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
mod transport;

use axum::Router;

use crate::state::AppState;

/// Builds public image routes.
pub(crate) fn routes() -> Router<AppState> {
    transport::routes()
}
