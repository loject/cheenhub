//! Регрессии базы сравнения после сохранения уровня журнала.

use super::*;

#[test]
fn successful_save_updates_baseline_and_allows_reverting_original_level() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let initial = HostLogSettingsResponse {
            min_level: Some(HostLogLevel::Warn),
            updated_at: None,
        };
        let saved_settings = Signal::new(initial.clone());
        let mut selected = Signal::new("debug".to_owned());
        let save_state = Signal::new(SaveState::Saving);
        let saved = HostLogSettingsResponse {
            min_level: Some(HostLogLevel::Debug),
            updated_at: Some("2026-10-04T00:00:00Z".into()),
        };

        complete_save(saved.clone(), selected, save_state, saved_settings);

        assert_eq!(saved_settings(), saved);
        assert_eq!(selected(), current_option(&saved_settings()));
        selected.set(current_option(&initial));
        assert_ne!(
            selected(),
            current_option(&saved_settings()),
            "returning to initial warn must enable another save"
        );
    });
}
