//! Компонент маршрута восстановления пароля.

use dioxus::prelude::*;

use crate::features::auth::ForgotPasswordPage;

#[component]
pub(crate) fn ForgotPassword() -> Element {
    rsx! {
        ForgotPasswordPage {}
    }
}
