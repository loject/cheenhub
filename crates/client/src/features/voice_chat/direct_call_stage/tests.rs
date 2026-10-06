use super::{DirectCallStageStatus, stage_state_content};

#[test]
fn keeps_connected_stage_free_for_peer_media() {
    assert!(stage_state_content(&DirectCallStageStatus::Connected, "Лиса").is_none());
}

#[test]
fn exposes_retry_only_for_terminal_media_error() {
    let connecting = stage_state_content(&DirectCallStageStatus::Connecting, "Лиса")
        .expect("connecting state should have explanatory content");
    let error = stage_state_content(
        &DirectCallStageStatus::Error {
            message: "Нет связи".to_owned(),
        },
        "Лиса",
    )
    .expect("error state should have explanatory content");

    assert!(!connecting.2);
    assert!(error.2);
    assert_eq!(error.1, "Нет связи");
}
