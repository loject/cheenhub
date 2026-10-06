//! Нажатия на настоящие кнопки VirtualDom без renderer и сериализации событий.

use dioxus::dioxus_core::ElementId;
use dioxus::html::geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint};
use dioxus::html::input_data::{MouseButton, MouseButtonSet};
use dioxus::html::*;
use dioxus::prelude::*;
use std::{any::Any, rc::Rc};

/// Отправляет нажатие кнопки через настоящий Dioxus event dispatch.
pub(super) fn click(dom: &VirtualDom, id: ElementId) {
    set_event_converter(Box::new(ClickEventConverter));
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::new(ClickData))) as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("click", event, id);
}

struct ClickData;

impl HasMouseData for ClickData {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl InteractionLocation for ClickData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(0.0, 0.0)
    }
    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(0.0, 0.0)
    }
    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(0.0, 0.0)
    }
}

impl InteractionElementOffset for ClickData {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(0.0, 0.0)
    }
}

impl ModifiersInteraction for ClickData {
    fn modifiers(&self) -> Modifiers {
        Modifiers::empty()
    }
}

impl PointerInteraction for ClickData {
    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::Primary)
    }
    fn held_buttons(&self) -> MouseButtonSet {
        MouseButtonSet::empty()
    }
}

// Интерфейс Dioxus требует все виды событий; эта suite использует только mouse/click.
// Незапланированное событие должно уронить тест, а не незаметно возвращать заглушку.
struct ClickEventConverter;

impl HtmlEventConverter for ClickEventConverter {
    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        assert!(event.downcast::<ClickData>().is_some());
        MouseData::new(ClickData)
    }
    fn convert_animation_data(&self, _event: &PlatformEventData) -> AnimationData {
        panic!("unexpected AnimationData in owner button test");
    }
    fn convert_before_input_data(&self, _event: &PlatformEventData) -> BeforeInputData {
        panic!("unexpected BeforeInputData in owner button test");
    }
    fn convert_cancel_data(&self, _event: &PlatformEventData) -> CancelData {
        panic!("unexpected CancelData in owner button test");
    }
    fn convert_clipboard_data(&self, _event: &PlatformEventData) -> ClipboardData {
        panic!("unexpected ClipboardData in owner button test");
    }
    fn convert_composition_data(&self, _event: &PlatformEventData) -> CompositionData {
        panic!("unexpected CompositionData in owner button test");
    }
    fn convert_drag_data(&self, _event: &PlatformEventData) -> DragData {
        panic!("unexpected DragData in owner button test");
    }
    fn convert_focus_data(&self, _event: &PlatformEventData) -> FocusData {
        panic!("unexpected FocusData in owner button test");
    }
    fn convert_form_data(&self, _event: &PlatformEventData) -> FormData {
        panic!("unexpected FormData in owner button test");
    }
    fn convert_image_data(&self, _event: &PlatformEventData) -> ImageData {
        panic!("unexpected ImageData in owner button test");
    }
    fn convert_keyboard_data(&self, _event: &PlatformEventData) -> KeyboardData {
        panic!("unexpected KeyboardData in owner button test");
    }
    fn convert_media_data(&self, _event: &PlatformEventData) -> MediaData {
        panic!("unexpected MediaData in owner button test");
    }
    fn convert_mounted_data(&self, _event: &PlatformEventData) -> MountedData {
        panic!("unexpected MountedData in owner button test");
    }
    fn convert_pointer_data(&self, _event: &PlatformEventData) -> PointerData {
        panic!("unexpected PointerData in owner button test");
    }
    fn convert_resize_data(&self, _event: &PlatformEventData) -> ResizeData {
        panic!("unexpected ResizeData in owner button test");
    }
    fn convert_scroll_data(&self, _event: &PlatformEventData) -> ScrollData {
        panic!("unexpected ScrollData in owner button test");
    }
    fn convert_selection_data(&self, _event: &PlatformEventData) -> SelectionData {
        panic!("unexpected SelectionData in owner button test");
    }
    fn convert_toggle_data(&self, _event: &PlatformEventData) -> ToggleData {
        panic!("unexpected ToggleData in owner button test");
    }
    fn convert_touch_data(&self, _event: &PlatformEventData) -> TouchData {
        panic!("unexpected TouchData in owner button test");
    }
    fn convert_transition_data(&self, _event: &PlatformEventData) -> TransitionData {
        panic!("unexpected TransitionData in owner button test");
    }
    fn convert_visible_data(&self, _event: &PlatformEventData) -> VisibleData {
        panic!("unexpected VisibleData in owner button test");
    }
    fn convert_wheel_data(&self, _event: &PlatformEventData) -> WheelData {
        panic!("unexpected WheelData in owner button test");
    }
}
