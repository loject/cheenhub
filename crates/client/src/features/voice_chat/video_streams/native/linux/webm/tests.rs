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
    assert!(first.bytes.windows(5).any(|b| b == b"V_VP9"));
    assert!(
        first
            .bytes
            .windows(KEY_HEADER.len())
            .any(|bytes| bytes == KEY_HEADER)
    );
    assert_eq!(first.dimensions.unwrap().display_width, 64);
    assert_eq!(
        next.bytes,
        [
            0x1f, 0x43, 0xb6, 0x75, 0x8b, 0xe7, 0x81, 33, 0xa3, 0x86, 0x81, 0, 0, 0, 0x86, 0xab
        ]
    );
    assert!(next.dimensions.is_none());
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
#[ignore = "Нужны установленные ffmpeg и ffprobe с libvpx-vp9; создаёт временный WebM"]
fn ffmpeg_decodes_muxed_key_and_delta_frames_with_rebased_timestamps() {
    use std::process::Command;

    let encoded = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=64x48:r=30",
            "-frames:v",
            "2",
            "-c:v",
            "libvpx-vp9",
            "-deadline",
            "realtime",
            "-cpu-used",
            "8",
            "-lag-in-frames",
            "0",
            "-f",
            "ivf",
            "pipe:1",
        ])
        .output()
        .expect("ffmpeg installed");
    assert!(
        encoded.status.success(),
        "{}",
        String::from_utf8_lossy(&encoded.stderr)
    );
    let mut input = &encoded.stdout[32..];
    let mut muxer = WebmMuxer::default();
    let mut webm = Vec::new();
    for index in 0..2 {
        let length = u32::from_le_bytes(input[..4].try_into().unwrap()) as usize;
        let frame = &input[12..12 + length];
        webm.extend(
            muxer
                .push(frame, 9_000_000 + index * 33_333, 33_333, index == 0)
                .unwrap()
                .unwrap()
                .bytes,
        );
        input = &input[12 + length..];
    }
    let path = std::env::temp_dir().join(format!("cheenhub-webm-{}.webm", uuid::Uuid::new_v4()));
    std::fs::write(&path, webm).unwrap();

    let decoded = Command::new("ffprobe")
        .args(["-v", "error", "-show_frames", "-of", "json"])
        .arg(&path)
        .output()
        .expect("ffprobe installed");
    std::fs::remove_file(path).unwrap();

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
