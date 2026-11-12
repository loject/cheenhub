use super::*;

#[test]
fn cancelled_start_runs_cleanup_when_backend_registers_it_late() {
    let control = ScreenShareStartControl::default();
    let stopped = Rc::new(std::cell::Cell::new(0));
    let cleanup_stopped = stopped.clone();

    control.cancel();
    control.register_cleanup(Rc::new(move || {
        cleanup_stopped.set(cleanup_stopped.get() + 1);
    }));

    assert_eq!(stopped.get(), 1);
    assert!(control.is_cancelled());
}

#[test]
fn repeated_start_cancellation_runs_registered_cleanup_once() {
    let control = ScreenShareStartControl::default();
    let stopped = Rc::new(std::cell::Cell::new(0));
    let cleanup_stopped = stopped.clone();
    control.register_cleanup(Rc::new(move || {
        cleanup_stopped.set(cleanup_stopped.get() + 1);
    }));

    control.cancel();
    control.cancel();

    assert_eq!(stopped.get(), 1);
}

#[test]
fn late_permission_response_releases_every_track_from_the_new_stream() {
    let control = ScreenShareStartControl::default();
    let video_stopped = Rc::new(std::cell::Cell::new(false));
    let audio_stopped = Rc::new(std::cell::Cell::new(false));
    let cleanup_video = video_stopped.clone();
    let cleanup_audio = audio_stopped.clone();

    control.cancel();
    control.register_cleanup(Rc::new(move || {
        cleanup_video.set(true);
        cleanup_audio.set(true);
    }));

    assert!(video_stopped.get());
    assert!(audio_stopped.get());
}

#[test]
fn twenty_capture_start_stop_cycles_release_each_capture_once() {
    let active_captures = Rc::new(std::cell::Cell::new(0));
    let released_captures = Rc::new(std::cell::Cell::new(0));

    for _ in 0..20 {
        let control = ScreenShareStartControl::default();
        active_captures.set(active_captures.get() + 1);
        let active = active_captures.clone();
        let released = released_captures.clone();
        control.register_cleanup(Rc::new(move || {
            active.set(active.get() - 1);
            released.set(released.get() + 1);
        }));

        control.cancel();
        control.cancel();
    }

    assert_eq!(active_captures.get(), 0);
    assert_eq!(released_captures.get(), 20);
}

#[test]
fn cancelling_old_start_does_not_release_the_replacement_capture() {
    let old_control = ScreenShareStartControl::default();
    let new_control = ScreenShareStartControl::default();
    let new_capture_released = Rc::new(std::cell::Cell::new(false));
    let release_new_capture = new_capture_released.clone();
    new_control.register_cleanup(Rc::new(move || release_new_capture.set(true)));

    old_control.cancel();

    assert!(!new_control.is_cancelled());
    assert!(!new_capture_released.get());

    new_control.cancel();

    assert!(new_capture_released.get());
}

#[test]
fn screen_quality_accepts_large_dimensions_and_frame_rates() {
    let config = ScreenShareConfig::default();
    for (width, height, max_fps) in [(1920, 1080, 30), (2560, 1440, 60), (3840, 2160, 60)] {
        let target = ScreenShareTargetQuality {
            width,
            height,
            max_fps,
            bitrate_bps: 1_500_000,
        };

        let actual = config
            .preset_for_target(&target)
            .expect("качество разрешено тарифом");

        assert_eq!(
            (actual.width, actual.height, actual.max_fps),
            (width, height, max_fps)
        );
    }
}

#[test]
fn browser_quality_target_keeps_selected_frame_rate_and_bitrate() {
    let config = ScreenShareConfig::default();
    let target = ScreenShareTargetQuality {
        width: 1_280,
        height: 720,
        max_fps: 15,
        bitrate_bps: 1_500_000,
    };

    let preset = config
        .preset_for_target(&target)
        .expect("доступный профиль");

    assert_eq!(
        (
            preset.width,
            preset.height,
            preset.max_fps,
            preset.bitrate_bps
        ),
        (1_280, 720, 15, 1_500_000)
    );
}

#[test]
fn browser_quality_target_rejects_zero_frame_rate() {
    let config = ScreenShareConfig::default();
    let target = ScreenShareTargetQuality {
        width: 1_920,
        height: 1_080,
        max_fps: 0,
        bitrate_bps: 1_500_000,
    };

    assert!(config.preset_for_target(&target).is_none());
}

#[test]
fn selected_bitrate_reaches_encoder_parameters_and_over_limit_is_rejected() {
    let config = ScreenShareConfig::default();
    let mut target = ScreenShareTargetQuality {
        width: 3840,
        height: 2160,
        max_fps: 60,
        bitrate_bps: 750_000,
    };

    assert_eq!(
        config.preset_for_target(&target).unwrap().bitrate_bps,
        750_000
    );
    target.bitrate_bps = 1_500_001;
    assert!(config.preset_for_target(&target).is_none());
}

#[test]
fn stopping_capture_revokes_an_already_queued_audio_frame() {
    let closed = Rc::new(std::cell::Cell::new(false));
    let frame = EncodedScreenAudioFrame {
        track_id: uuid::Uuid::new_v4(),
        sequence: 0,
        timestamp_us: 0,
        duration_us: 20_000,
        channels: 2,
        bytes: vec![1],
        closed: closed.clone(),
    };
    assert!(frame.can_send());

    closed.set(true);

    assert!(!frame.can_send());
}

#[test]
fn automatic_quality_keeps_hd_source_dimensions() {
    let spec = ScreenShareConfig::default().preset_for_capture(1366, 768);
    assert_eq!((spec.width, spec.height, spec.max_fps), (1366, 768, 30));
}

#[test]
fn automatic_quality_keeps_large_source_dimensions() {
    let spec = ScreenShareConfig::default().preset_for_capture(2560, 1440);
    assert_eq!((spec.width, spec.height, spec.max_fps), (2560, 1440, 30));
}

#[test]
fn automatic_quality_preserves_selected_bitrate() {
    let config = ScreenShareConfig {
        automatic_bitrate_bps: 750_000,
        ..Default::default()
    };

    let settings = config.preset_for_capture(3840, 2160);

    assert_eq!(encoder_config(settings).bitrate_bps, 750_000);
}

#[test]
fn capture_preserves_source_aspect_within_the_selected_quality() {
    for (source, limit, expected) in [
        ((3440, 1440), (1920, 1080), (1920, 804)),
        ((2520, 1080), (1280, 720), (1280, 549)),
        ((2560, 1440), (1920, 1080), (1920, 1080)),
        ((1080, 1920), (1280, 720), (405, 720)),
        ((640, 480), (1280, 720), (640, 480)),
        ((1366, 768), (1280, 720), (1280, 720)),
        ((1920, 1080), (3840, 2160), (1920, 1080)),
        ((2560, 1440), (3840, 2160), (2560, 1440)),
        ((3840, 2160), (3840, 2160), (3840, 2160)),
        ((3840, 2160), (1920, 1080), (1920, 1080)),
        ((0, 0), (1280, 720), (1, 1)),
        ((u32::MAX, u32::MAX), (1920, 1080), (1080, 1080)),
    ] {
        let preset = VideoEncodingSettings {
            source: VideoStreamSource::ScreenShare,
            width: limit.0,
            height: limit.1,
            max_fps: 30,
            bitrate_bps: 1_500_000,
        };

        let fitted = fit_capture_preset(source.0, source.1, preset);

        assert_eq!((fitted.width, fitted.height), expected, "{source:?}");
        let encoder = encoder_config(fitted);
        assert_eq!((encoder.width, encoder.height), expected, "{source:?}");
        assert_eq!(fitted.max_fps, preset.max_fps);
        assert_eq!(fitted.bitrate_bps, preset.bitrate_bps);
    }
}
