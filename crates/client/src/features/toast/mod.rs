//! Фича глобальных toast-уведомлений.

mod provider;
mod timer;
mod update_available;

pub(crate) use provider::{ToastHandle, ToastProvider};
pub(crate) use update_available::{
    UpdateAvailableToast, UpdateAvailableToastActions, UpdateAvailableToastContent,
    UpdateToastDeferralOption,
};
