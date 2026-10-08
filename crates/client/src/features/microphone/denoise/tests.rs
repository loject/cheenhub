use super::*;

#[test]
fn selected_processor_accepts_native_ten_millisecond_pcm() {
    let mut processor = Processor::new();
    let mut frame = [0.25_f32; 480];
    let original = frame;
    processor.process(&mut frame).unwrap();
    assert_eq!(frame, original);
    processor.process(&mut frame).unwrap();
    assert!(frame.iter().all(|sample| sample.is_finite()));
    if !Processor::available() {
        assert_eq!(frame, original);
    }
}

#[test]
fn selected_processor_handles_invalid_pcm_according_to_availability() {
    let mut processor = Processor::new();
    let available = Processor::available();

    assert_eq!(processor.process(&mut [0.0; 479]).is_err(), available);
    let mut frame = [0.0_f32; 480];
    frame[10] = f32::NAN;
    assert_eq!(processor.process(&mut frame).is_err(), available);
}
