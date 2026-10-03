use std::{cell::Cell, future::Future, pin::Pin};

use dioxus::html::{MountedResult, RenderedElementBacking};
use dioxus::prelude::dioxus_elements::geometry::{PixelsRect, PixelsSize};
use futures_util::FutureExt;

use super::*;

struct Element {
    top: f64,
    height: f64,
    scroll_y: Rc<Cell<f64>>,
    during_measure: Option<Rc<dyn Fn()>>,
    measure_error: bool,
}

impl RenderedElementBacking for Element {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
        if let Some(callback) = &self.during_measure {
            callback();
        }
        if self.measure_error {
            return Box::pin(async { Err(dioxus::html::MountedError::NotSupported) });
        }
        let rect = PixelsRect::from_size(PixelsSize::new(400.0, self.height))
            .translate(PixelsVector2D::new(0.0, self.top));
        Box::pin(async move { Ok(rect) })
    }

    fn get_scroll_offset(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsVector2D>>>> {
        let y = self.scroll_y.get();
        Box::pin(async move { Ok(PixelsVector2D::new(0.0, y)) })
    }

    fn scroll(
        &self,
        coordinates: PixelsVector2D,
        _behavior: ScrollBehavior,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.scroll_y.set(coordinates.y);
        Box::pin(async { Ok(()) })
    }
}

fn element(top: f64, height: f64, scroll_y: Rc<Cell<f64>>) -> Rc<MountedData> {
    Rc::new(MountedData::new(Element {
        top,
        height,
        scroll_y,
        during_measure: None,
        measure_error: false,
    }))
}

fn registry(rows: &[(&str, Rc<MountedData>)]) -> AnchorElements {
    AnchorElements {
        rows: Signal::new(
            rows.iter()
                .map(|(id, row)| ((*id).to_owned(), row.clone()))
                .collect(),
        ),
        revision: Signal::new(0),
    }
}

fn message(id: &str) -> TextChatMessage {
    TextChatMessage {
        id: id.into(),
        server_id: "server".into(),
        room_id: "room".into(),
        author_user_id: "author".into(),
        author_nickname: "Автор".into(),
        author_avatar_url: None,
        body: id.into(),
        attachments: Vec::new(),
        delivery_status: None,
        created_at: "2026-10-02T00:00:00Z".into(),
    }
}

#[test]
fn measured_anchor_restores_reading_inside_a_long_message_after_layout_changes() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let y = Rc::new(Cell::new(2000.0));
        let list = element(100.0, 500.0, y.clone());
        let mut elements = registry(&[("long", element(-640.0, 1800.0, y.clone()))]);
        let anchor = capture_anchor(
            list.clone(),
            elements,
            &[message("previous"), message("long"), message("next")],
        )
        .now_or_never()
        .unwrap()
        .unwrap();
        assert_eq!(anchor.message_id, "long");
        assert_eq!(anchor.inside_y, 740.0);
        assert_eq!(anchor.preceding_ids, ["previous"]);
        elements
            .rows
            .write()
            .insert("long".into(), element(-300.0, 1800.0, y.clone()));
        assert_eq!(
            restore_anchor(list, elements, &anchor).now_or_never(),
            Some(true)
        );
        assert_eq!(y.get(), 2340.0);
    });
}

#[test]
fn manual_scroll_during_measurement_cancels_restore_without_overwriting_position() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let y = Rc::new(Cell::new(2000.0));
        let list = element(100.0, 500.0, y.clone());
        let mut elements = registry(&[]);
        let row = Rc::new(MountedData::new(Element {
            top: -300.0,
            height: 1800.0,
            scroll_y: y.clone(),
            during_measure: Some(Rc::new(move || elements.interacted())),
            measure_error: false,
        }));
        elements.rows.write().insert("long".into(), row);
        let anchor = ScrollAnchor {
            message_id: "long".into(),
            inside_y: 740.0,
            preceding_ids: vec![],
            revision: 0,
        };
        assert_eq!(
            restore_anchor(list, elements, &anchor).now_or_never(),
            Some(false)
        );
        assert_eq!(y.get(), 2000.0);
    });
}

#[test]
fn target_group_is_pinned_by_message_id_even_when_group_key_changes() {
    let anchor = ScrollAnchor {
        message_id: "long".into(),
        inside_y: 740.0,
        preceding_ids: vec![],
        revision: 0,
    };
    assert!(targets_group(
        Some(ScrollCommand::Restore {
            anchor: anchor.clone()
        }),
        &[message("new-group-first"), message("long")]
    ));
    assert!(!targets_group(
        Some(ScrollCommand::Restore { anchor }),
        &[message("another")]
    ));
}

#[test]
fn deletion_during_measurement_retries_the_surviving_predecessor() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let y = Rc::new(Cell::new(2000.0));
        let list = element(100.0, 500.0, y.clone());
        let current_ids = Signal::new(vec!["previous".to_owned(), "long".to_owned(), "next".to_owned()]);
        let row = Rc::new(MountedData::new(Element {
            top: -300.0, height: 1800.0, scroll_y: y.clone(), measure_error: true,
            during_measure: Some(Rc::new(move || { let mut ids = current_ids; ids.write().retain(|id| id != "long"); })),
        }));
        let elements = registry(&[("long", row)]);
        let anchor = ScrollAnchor { message_id: "long".into(), inside_y: 740.0, preceding_ids: vec!["previous".into()], revision: 0 };
        assert_eq!(restore_anchor(list, elements, &anchor).now_or_never(), Some(false));
        let next = remaining_restore(ScrollCommand::Restore { anchor }, &current_ids.peek());
        assert!(matches!(next, Some(ScrollCommand::Restore { anchor }) if anchor.message_id == "previous" && anchor.inside_y == 0.0));
        assert_eq!(y.get(), 2000.0);
    });
}
