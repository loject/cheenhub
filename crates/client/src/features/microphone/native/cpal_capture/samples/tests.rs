use super::*;

#[test]
fn downmix_averages_interleaved_channels() {
    let samples = downmix_to_mono(&[1.0_f32, -1.0, 0.25, 0.75], 2);

    assert_eq!(samples, vec![0.0, 0.5]);
}
