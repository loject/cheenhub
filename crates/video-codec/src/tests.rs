use super::*;

#[test]
fn rejects_invalid_encoder_configuration_before_opening_a_device() {
    for (width, height, fps, bitrate) in [
        (0, 64, 30, 1000),
        (63, 64, 30, 1000),
        (64, 64, 0, 1000),
        (64, 64, 30, 0),
        (u32::MAX, 64, 30, 1000),
    ] {
        assert!(Vp9Encoder::new(width, height, fps, bitrate).is_err());
    }
}

#[test]
fn webm_contains_vp9_track_and_rebases_timestamps() {
    let mut muxer = WebmMuxer::new(64, 48, 33_333).unwrap();
    let key = [0x82, 0x49, 0x83, 0x42, 0x00, 0x03, 0xf0, 0x02, 0xf6];

    let first = muxer.push(&key, 9_000_000, 33_333, true).unwrap();
    let second = muxer.push(&[0x86, 0xab], 9_033_333, 33_333, false).unwrap();

    assert!(first.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]));
    assert!(first.windows(5).any(|value| value == b"V_VP9"));
    assert!(first.windows(key.len()).any(|value| value == key));
    assert!(!second.is_empty());
    assert!(muxer.push(&[], 9_066_666, 33_333, false).is_err());
    assert!(muxer.push(&key, 8_000_000, 33_333, true).is_err());
}

#[test]
fn webm_rejects_delta_frame_before_first_key_frame() {
    let mut muxer = WebmMuxer::new(64, 48, 33_333).unwrap();

    assert!(muxer.push(&[0x86], 0, 33_333, false).is_err());
}

#[test]
fn webm_accepts_unspecified_duration_on_first_and_following_frames() {
    let mut muxer = WebmMuxer::new(64, 48, 0).unwrap();
    let key = [0x82, 0x49, 0x83, 0x42, 0x00, 0x03, 0xf0, 0x02, 0xf6];

    let first = muxer.push(&key, 9_000_000, 0, true).unwrap();
    let next = muxer.push(&[0x86, 0xab], 9_033_333, 0, false).unwrap();

    assert!(first.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]));
    assert!(!next.is_empty());
}

#[test]
fn webm_accepts_unspecified_duration_after_known_duration() {
    let mut muxer = WebmMuxer::new(64, 48, 33_333).unwrap();
    let key = [0x82, 0x49, 0x83, 0x42, 0x00, 0x03, 0xf0, 0x02, 0xf6];
    muxer.push(&key, 9_000_000, 33_333, true).unwrap();

    let next = muxer.push(&[0x86, 0xab], 9_033_333, 0, false).unwrap();

    assert!(!next.is_empty());
}

#[test]
fn accepts_odd_dimensions_of_received_vp9_frames() {
    assert!(WebmMuxer::new(63, 47, 33_333).is_ok());
}

#[test]
#[ignore = "Нужен установленный ffprobe с VP9-декодером; создаёт временный WebM"]
fn ffprobe_decodes_live_webm_with_rebased_timestamps() {
    let mut input = &include_bytes!("tests/fixtures/red.ivf")[32..];
    let mut muxer = WebmMuxer::new(64, 48, 33_333).unwrap();
    let mut output = Vec::new();
    for index in 0..2 {
        let size = u32::from_le_bytes(input[..4].try_into().unwrap()) as usize;
        output.extend(
            muxer
                .push(
                    &input[12..12 + size],
                    9_000_000 + index * 33_333,
                    33_333,
                    index == 0,
                )
                .unwrap(),
        );
        input = &input[12 + size..];
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cheenhub-codec-{}-{unique}.webm",
        std::process::id()
    ));
    std::fs::write(&path, output).unwrap();

    let result = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-show_frames", "-of", "json"])
        .arg(&path)
        .output();
    std::fs::remove_file(&path).unwrap();
    let result = result.expect("ffprobe installed");

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let frames = json["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0]["width"], 64);
    assert_eq!(frames[0]["height"], 48);
    assert_eq!(frames[0]["key_frame"], 1);
    assert_eq!(frames[1]["key_frame"], 0);
    assert_eq!(frames[0]["pts_time"], "0.000000");
    assert_eq!(frames[1]["pts_time"], "0.033000");
}
