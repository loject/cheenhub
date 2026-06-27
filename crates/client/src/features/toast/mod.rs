//! Фича глобальных toast-уведомлений.

mod provider;

pub(crate) use provider::{
    ToastHandle, ToastProvider, UpdateAvailableToast, UpdateAvailableToastActions,
    UpdateAvailableToastContent, UpdateToastDeferralOption,
};
