use super::*;

const KEY_HEADER: &[u8] = &[0x82, 0x49, 0x83, 0x42, 0x00, 0x03, 0xf0, 0x02, 0xf6];

#[test]
fn waits_for_key_frame_then_emits_header_and_complete_cluster() {
    let mut muxer = WebmMuxer::default();

    assert!(
        muxer
            .push(&[0x86], 1_000_000, 33_333, false)
            .unwrap()
            .is_none()
    );
    let first = muxer
        .push(KEY_HEADER, 9_000_000, 33_333, true)
        .unwrap()
        .unwrap();
    let next = muxer
        .push(&[0x86, 0xab], 9_033_333, 33_333, false)
        .unwrap()
        .unwrap();

    assert!(first.bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]));
    assert!(
        first.bytes.len() >= 4096,
        "первый фрагмент должен заполнить read buffer плеера"
    );
    assert!(first.bytes.windows(5).any(|b| b == b"V_VP9"));
    assert!(
        first
            .bytes
            .windows(KEY_HEADER.len())
            .any(|bytes| bytes == KEY_HEADER)
    );
    assert_eq!(first.dimensions.unwrap().display_width, 64);
    assert!(next.bytes.windows(2).any(|bytes| bytes == [0x86, 0xab]));
    assert!(next.dimensions.is_none());
}

#[test]
fn browser_frames_with_unspecified_duration_start_and_continue_playback() {
    let mut muxer = WebmMuxer::default();

    let first = muxer.push(KEY_HEADER, 9_000_000, 0, true).unwrap().unwrap();
    let next = muxer
        .push(&[0x86, 0xab], 9_033_333, 0, false)
        .unwrap()
        .unwrap();

    assert!(first.bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]));
    assert!(first.bytes.len() >= 4096);
    assert!(!next.bytes.is_empty());
}

#[test]
fn rejects_invalid_key_frame_without_poisoning_startup() {
    let mut muxer = WebmMuxer::default();

    assert!(muxer.push(&[0], 10_000, 33_333, true).is_err());
    let valid = muxer
        .push(KEY_HEADER, 5_000, 33_333, true)
        .unwrap()
        .unwrap();

    assert!(valid.bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]));
}

#[test]
fn drops_late_frames_without_interrupting_the_live_stream() {
    let mut muxer = WebmMuxer::default();
    muxer.push(KEY_HEADER, 100_000, 33_333, true).unwrap();

    assert!(
        muxer
            .push(&[0x86], 90_000, 33_333, false)
            .unwrap()
            .is_none()
    );
    assert!(muxer.push(&[], 133_333, 33_333, false).is_err());
    assert!(
        muxer
            .push(&[0x86], 133_333, 33_333, false)
            .unwrap()
            .is_some()
    );
}

#[test]
#[ignore = "Нужен установленный ffprobe с VP9-декодером; создаёт временный WebM"]
fn ffmpeg_decodes_muxed_key_and_delta_frames_with_rebased_timestamps() {
    assert_decoded_frames(33_333);
}

#[test]
#[ignore = "Нужен установленный ffprobe с VP9-декодером; создаёт временный WebM"]
fn ffmpeg_decodes_browser_frames_with_unspecified_duration() {
    assert_decoded_frames(0);
}

fn assert_decoded_frames(duration_us: u32) {
    use std::process::Command;

    let webm = muxed_fixture(duration_us);
    let path = std::env::temp_dir().join(format!("cheenhub-webm-{}.webm", uuid::Uuid::new_v4()));
    std::fs::write(&path, webm).unwrap();

    let decoded = Command::new("ffprobe")
        .args(["-v", "error", "-show_frames", "-of", "json"])
        .arg(&path)
        .output();
    std::fs::remove_file(path).unwrap();
    let decoded = decoded.expect("ffprobe installed");

    assert!(
        decoded.status.success(),
        "{}",
        String::from_utf8_lossy(&decoded.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&decoded.stdout).unwrap();
    let frames = json["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0]["width"], 64);
    assert_eq!(frames[0]["height"], 48);
    assert_eq!(frames[0]["key_frame"], 1);
    assert_eq!(frames[1]["key_frame"], 0);
    assert_eq!(frames[0]["pts_time"], "0.000000");
    assert_eq!(frames[1]["pts_time"], "0.033000");
}

fn muxed_fixture(duration_us: u32) -> Vec<u8> {
    let mut input = &include_bytes!("tests/fixtures/red.ivf")[32..];
    let mut muxer = WebmMuxer::default();
    let mut webm = Vec::new();
    for index in 0..2 {
        let length = u32::from_le_bytes(input[..4].try_into().unwrap()) as usize;
        let frame = &input[12..12 + length];
        webm.extend(
            muxer
                .push(frame, 9_000_000 + index * 33_333, duration_us, index == 0)
                .unwrap()
                .unwrap()
                .bytes,
        );
        input = &input[12 + length..];
    }
    webm
}

#[test]
#[ignore = "Нужен gst-launch-1.0 с matroskademux и vp9dec; создаёт временный WebM"]
fn gstreamer_decodes_browser_frames_with_unspecified_duration() {
    let path =
        std::env::temp_dir().join(format!("cheenhub-gstreamer-{}.webm", uuid::Uuid::new_v4()));
    let registry = path.with_extension("registry.bin");
    std::fs::write(&path, muxed_fixture(0)).unwrap();

    let decoded = std::process::Command::new("gst-launch-1.0")
        .env("GST_REGISTRY", &registry)
        .args(["-q", "filesrc"])
        .arg(format!("location={}", path.display()))
        .args([
            "!",
            "matroskademux",
            "!",
            "vp9dec",
            "!",
            "video/x-raw,width=64,height=48",
            "!",
            "fakesink",
            "sync=false",
        ])
        .output();
    std::fs::remove_file(path).unwrap();
    if registry.exists() {
        std::fs::remove_file(registry).unwrap();
    }
    let decoded = decoded.expect("gst-launch-1.0 installed");

    assert!(
        decoded.status.success(),
        "{}",
        String::from_utf8_lossy(&decoded.stderr)
    );
}
