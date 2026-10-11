use super::*;

#[test]
fn speech_probability_matches_rnnoise_during_warmup_and_processing() {
    let mut processor = RnnoiseProcessor::new();
    let mut reference = DenoiseState::new();

    for _ in 0..3 {
        let input = [0.25 * 32_768.0; FRAME_SAMPLES];
        let mut output = [0.0; FRAME_SAMPLES];
        let expected_probability = reference.process_frame(&mut output, &input);
        let mut frame = [0.25; FRAME_SAMPLES];

        let probability = processor.process(&mut frame).unwrap();

        assert_eq!(probability, Some(expected_probability));
        assert!(frame.iter().all(|sample| sample.is_finite()));
    }
}

#[test]
fn invalid_pcm_does_not_produce_speech_probability() {
    let mut processor = RnnoiseProcessor::new();
    let mut frame = [0.0; FRAME_SAMPLES];
    frame[10] = f32::NAN;

    assert!(processor.process(&mut frame).is_err());
    assert!(processor.process(&mut [0.0; FRAME_SAMPLES - 1]).is_err());
}
