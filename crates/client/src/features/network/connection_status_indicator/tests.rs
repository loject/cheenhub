use super::{PingSample, graph_points};

#[test]
fn graph_handles_a_minute_of_voice_frequency_samples() {
    let samples = (0..=80)
        .map(|index| PingSample {
            received_at_ms: index * 750,
            rtt_ms: 20.0 + (index % 7) as f64,
            jitter_ms: 1.0,
        })
        .collect::<Vec<_>>();

    let points = graph_points(&samples);

    assert_eq!(points.split_whitespace().count(), samples.len());
    assert!(!points.contains("NaN"));
    assert!(!points.contains("inf"));
}

#[test]
fn graph_handles_equal_sample_timestamps() {
    let samples = vec![
        PingSample {
            received_at_ms: 1_000,
            rtt_ms: 20.0,
            jitter_ms: 0.0,
        },
        PingSample {
            received_at_ms: 1_000,
            rtt_ms: 24.0,
            jitter_ms: 4.0,
        },
    ];

    let points = graph_points(&samples);

    assert_eq!(points.split_whitespace().count(), 2);
    assert!(!points.contains("NaN"));
    assert!(!points.contains("inf"));
}

#[test]
fn connection_color_tracks_ping_independently_of_transport() {
    use crate::features::realtime::{
        RealtimeConnectionStatus as Status, RealtimeTransportKind as Transport,
    };

    for transport in [Transport::WebTransport, Transport::WebSocketFallback] {
        let status = Status::Connected(transport);
        assert_eq!(super::connection_color(status, Some(0.0)), (52, 211, 153));
        assert_eq!(super::connection_color(status, Some(30.0)), (52, 211, 153));
        assert_eq!(super::connection_color(status, Some(90.0)), (152, 179, 107));
        assert_eq!(super::connection_color(status, Some(150.0)), (251, 146, 60));
        assert_eq!(
            super::connection_color(status, Some(1000.0)),
            (251, 146, 60)
        );
        assert_eq!(super::connection_color(status, None), (161, 161, 170));
    }
}

#[test]
fn disconnected_is_red_and_connecting_is_neutral_despite_previous_ping() {
    use crate::features::realtime::RealtimeConnectionStatus as Status;

    assert_eq!(
        super::connection_color(Status::Disconnected, Some(10.0)),
        (248, 113, 113)
    );
    for status in [
        Status::ConnectingWebTransport,
        Status::ConnectingWebSocketFallback,
    ] {
        assert_eq!(super::connection_color(status, Some(10.0)), (161, 161, 170));
    }
}
