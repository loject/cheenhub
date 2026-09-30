//! Управление возвратом фокуса в поле ввода чата.

use std::{cell::Cell, rc::Rc};

use dioxus::prelude::*;

pub(super) fn restore_compose_input_focus(
    input_element: Signal<Option<Rc<MountedData>>>,
    refocus_requested: Signal<bool>,
    component_current: Rc<Cell<bool>>,
) {
    if !should_refocus(component_current.get(), refocus_requested()) {
        return;
    }

    let Some(element) = input_element.cloned() else {
        return;
    };

    spawn(async move {
        if !should_refocus(component_current.get(), refocus_requested()) {
            return;
        }

        if let Err(error) = element.set_focus(true).await {
            debug!(?error, "failed to restore text chat input focus");
        }
    });
}

fn should_refocus(component_current: bool, refocus_requested: bool) -> bool {
    component_current && refocus_requested
}

#[cfg(test)]
mod tests {
    use super::should_refocus;

    #[test]
    fn refocus_requires_an_active_component_and_submit_intent() {
        assert!(should_refocus(true, true));
        assert!(!should_refocus(false, true));
        assert!(!should_refocus(true, false));
    }
}
