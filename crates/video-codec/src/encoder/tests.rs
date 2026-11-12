use super::*;

#[test]
fn copies_i420_luma_and_interleaves_chroma_respecting_nv12_stride() {
    let mut frame = frame::Video::new(format::Pixel::NV12, 4, 2);
    let i420 = [16, 17, 18, 19, 20, 21, 22, 23, 90, 91, 140, 141];

    write_i420(&mut frame, &i420).unwrap();

    assert_eq!(&frame.data(0)[..4], &[16, 17, 18, 19]);
    let stride = frame.stride(0);
    assert_eq!(&frame.data(0)[stride..stride + 4], &[20, 21, 22, 23]);
    assert_eq!(&frame.data(1)[..4], &[90, 140, 91, 141]);
}

#[test]
fn rejects_truncated_i420_before_copying_any_plane() {
    let mut frame = frame::Video::new(format::Pixel::NV12, 4, 2);

    assert!(write_i420(&mut frame, &[16; 11]).is_err());
}

#[test]
fn falls_back_to_software_when_hardware_encoder_is_absent() {
    let candidates = platform::candidates();
    let mut unavailable = candidates[0];
    unavailable.name = "cheenhub_missing_hardware_encoder";
    let software = *candidates.last().unwrap();
    let candidates = [unavailable, software];

    let mut encoder = Vp9Encoder::with_candidates(64, 48, 30, 300_000, &candidates).unwrap();
    let packets = encoder
        .encode(&vec![128; 64 * 48 * 3 / 2], 1_000_000, true)
        .unwrap();

    assert!(!encoder.hardware);
    assert_eq!(packets.len(), 1);
    assert!(packets[0].key_frame);
    assert_eq!(packets[0].timestamp_us, 1_000_000);
}

#[test]
fn software_vp9_encodes_without_a_gpu_and_preserves_timestamps() {
    let software = platform::candidates().last().unwrap();
    assert_eq!(software.name, "libvpx-vp9");
    let mut encoder =
        Vp9Encoder::with_candidates(64, 48, 30, 300_000, std::slice::from_ref(software)).unwrap();
    let mut i420 = vec![16; 64 * 48 * 3 / 2];
    i420[64 * 48..].fill(128);

    let first = encoder.encode(&i420, 1_000_000, true).unwrap();
    let next = encoder.encode(&i420, 1_033_333, false).unwrap();
    let forced = encoder.encode(&i420, 1_066_666, true).unwrap();

    assert_eq!(first.len(), 1);
    assert!(first[0].key_frame);
    assert_eq!(first[0].timestamp_us, 1_000_000);
    assert_eq!(next.len(), 1);
    assert!(!next[0].key_frame);
    assert_eq!(next[0].timestamp_us, 1_033_333);
    assert_eq!(forced.len(), 1);
    assert!(forced[0].key_frame);
    assert_eq!(forced[0].timestamp_us, 1_066_666);
}

#[test]
#[ignore = "Нужен установленный ffprobe с VP9-декодером; создаёт временный WebM"]
fn software_encoder_output_decodes_as_vp9_webm() {
    let software = platform::candidates().last().unwrap();
    let mut encoder =
        Vp9Encoder::with_candidates(64, 48, 30, 300_000, std::slice::from_ref(software)).unwrap();
    let mut muxer = crate::WebmMuxer::new(64, 48, 33_333).unwrap();
    let mut i420 = vec![16; 64 * 48 * 3 / 2];
    i420[64 * 48..].fill(128);
    let mut bytes = Vec::new();
    for index in 0..2 {
        for packet in encoder
            .encode(&i420, 9_000_000 + index * 33_333, index == 0)
            .unwrap()
        {
            bytes.extend(
                muxer
                    .push(&packet.bytes, packet.timestamp_us, 33_333, packet.key_frame)
                    .unwrap(),
            );
        }
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cheenhub-software-{}-{unique}.webm",
        std::process::id()
    ));
    std::fs::write(&path, bytes).unwrap();

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

#[test]
#[ignore = "Нужен GPU с аппаратным VP9 encoder и установленный драйвер; программный резерв отключён"]
fn hardware_encoder_produces_key_frame_and_preserves_timestamp() {
    let candidates = platform::candidates();
    let mut encoder =
        Vp9Encoder::with_candidates(128, 128, 30, 300_000, &candidates[..candidates.len() - 1])
            .unwrap();
    assert!(encoder.hardware);
    let mut i420 = vec![16; 128 * 128 * 3 / 2];
    i420[128 * 128..].fill(128);
    let mut packets = Vec::new();

    for index in 0..8 {
        packets.extend(
            encoder
                .encode(&i420, 1_000_000 + index * 33_333, index == 0)
                .unwrap(),
        );
    }

    assert!(!packets.is_empty());
    assert!(packets[0].key_frame);
    assert_eq!(packets[0].timestamp_us, 1_000_000);
    assert!(!packets[0].bytes.is_empty());
}

#[test]
fn finishing_delayed_software_encoder_returns_buffered_packets() {
    let codec = encoder::find_by_name("libvpx-vp9").unwrap();
    let mut video = codec::Context::new_with_codec(codec)
        .encoder()
        .video()
        .unwrap();
    video.set_width(64);
    video.set_height(48);
    video.set_format(format::Pixel::YUV420P);
    video.set_bit_rate(300_000);
    video.set_time_base((1, 1_000_000));
    video.set_frame_rate(Some((30, 1)));
    let mut options = Dictionary::new();
    options.set("lag-in-frames", "10");
    options.set("deadline", "good");
    let encoder = video.open_as_with(codec, options).unwrap();
    let software = frame::Video::new(format::Pixel::YUV420P, 64, 48);
    let mut encoder = Vp9Encoder {
        encoder,
        software,
        input_len: 64 * 48 * 3 / 2,
        hardware: false,
    };
    let input = vec![128; 64 * 48 * 3 / 2];
    let mut output = Vec::new();
    for index in 0..4 {
        output.extend(encoder.encode(&input, index * 33_333, index == 0).unwrap());
    }
    assert!(output.len() < 4, "кодировщик должен задержать кадры");

    output.extend(encoder.finish().unwrap());

    assert_eq!(output.len(), 4);
    assert_eq!(
        output
            .iter()
            .map(|packet| packet.timestamp_us)
            .collect::<Vec<_>>(),
        vec![0, 33_333, 66_666, 99_999]
    );
}
