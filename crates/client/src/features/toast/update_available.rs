//! Данные toast-уведомления о доступном обновлении.

use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct UpdateAvailableToast {
    pub(super) current_version: String,
    pub(super) update_version: String,
    pub(super) title: Option<String>,
    pub(super) primary_label: String,
    pub(super) primary_disabled: bool,
    pub(super) deferral_options: Vec<UpdateToastDeferralOption>,
    pub(super) selected_deferral_value: String,
    pub(super) on_install: Rc<dyn Fn()>,
    pub(super) on_quick_dismiss: Rc<dyn Fn()>,
    pub(super) on_defer: Rc<dyn Fn(String)>,
}

impl UpdateAvailableToast {
    /// Создает данные toast-уведомления о доступном обновлении.
    pub(crate) fn new(
        content: UpdateAvailableToastContent,
        actions: UpdateAvailableToastActions,
    ) -> Self {
        Self {
            current_version: content.current_version,
            update_version: content.update_version,
            title: content.title,
            primary_label: content.primary_label,
            primary_disabled: content.primary_disabled,
            deferral_options: content.deferral_options,
            selected_deferral_value: content.default_deferral_value,
            on_install: actions.on_install,
            on_quick_dismiss: actions.on_quick_dismiss,
            on_defer: actions.on_defer,
        }
    }
}

/// Текстовые данные toast-уведомления о доступном обновлении.
pub(crate) struct UpdateAvailableToastContent {
    current_version: String,
    update_version: String,
    title: Option<String>,
    primary_label: String,
    primary_disabled: bool,
    deferral_options: Vec<UpdateToastDeferralOption>,
    default_deferral_value: String,
}

impl UpdateAvailableToastContent {
    /// Создает текстовые данные update-toast.
    pub(crate) fn new(
        current_version: impl Into<String>,
        update_version: impl Into<String>,
        title: Option<String>,
        primary_label: impl Into<String>,
        primary_disabled: bool,
        deferral_options: Vec<UpdateToastDeferralOption>,
        default_deferral_value: impl Into<String>,
    ) -> Self {
        Self {
            current_version: current_version.into(),
            update_version: update_version.into(),
            title,
            primary_label: primary_label.into(),
            primary_disabled,
            deferral_options,
            default_deferral_value: default_deferral_value.into(),
        }
    }
}

/// Действия toast-уведомления о доступном обновлении.
pub(crate) struct UpdateAvailableToastActions {
    on_install: Rc<dyn Fn()>,
    on_quick_dismiss: Rc<dyn Fn()>,
    on_defer: Rc<dyn Fn(String)>,
}

impl UpdateAvailableToastActions {
    /// Создает callbacks для update-toast.
    pub(crate) fn new(
        on_install: impl Fn() + 'static,
        on_quick_dismiss: impl Fn() + 'static,
        on_defer: impl Fn(String) + 'static,
    ) -> Self {
        Self {
            on_install: Rc::new(on_install),
            on_quick_dismiss: Rc::new(on_quick_dismiss),
            on_defer: Rc::new(on_defer),
        }
    }
}

/// Пункт выбора времени отсрочки внутри update-toast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UpdateToastDeferralOption {
    pub(super) value: String,
    pub(super) label: String,
}

impl UpdateToastDeferralOption {
    /// Создает пункт выбора времени отсрочки.
    pub(crate) fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}
