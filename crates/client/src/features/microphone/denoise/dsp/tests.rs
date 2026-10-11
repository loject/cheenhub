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

#[test]
fn stream_processing_can_switch_without_changing_pcm_when_disabled() {
    let mut processor = StreamProcessor::new();
    let mut frame = [0.25; 960];

    assert_eq!(processor.process(&mut frame, false).unwrap(), None);
    assert_eq!(frame, [0.25; 960]);
    assert_eq!(
        processor.process(&mut frame, true).unwrap().is_some(),
        Processor::available()
    );
    frame.fill(0.25);
    assert_eq!(processor.process(&mut frame, false).unwrap(), None);
    assert_eq!(frame, [0.25; 960]);
    assert_eq!(
        processor.process(&mut frame, true).unwrap().is_some(),
        Processor::available()
    );
}

#[test]
fn invalid_stream_frame_preserves_input_on_failure() {
    let mut processor = StreamProcessor::new();
    let mut frame = [0.25; 481];

    assert_eq!(
        processor.process(&mut frame, true).is_err(),
        Processor::available()
    );
    assert_eq!(frame, [0.25; 481]);
}

#[test]
fn later_invalid_frame_preserves_entire_chunk_and_retry_recovers() {
    let mut processor = StreamProcessor::new();
    let mut chunk = [0.25; 960];
    chunk[900] = f32::NAN;
    let original = chunk.map(f32::to_bits);

    let result = processor.process(&mut chunk, true);

    assert_eq!(result.is_err(), Processor::available());
    assert_eq!(chunk.map(f32::to_bits), original);
    chunk.fill(0.25);
    assert_eq!(
        processor.process(&mut chunk, true).unwrap().is_some(),
        Processor::available()
    );
}

#[test]
fn stream_reuses_scratch_storage_for_equal_and_smaller_chunks() {
    if !Processor::available() {
        return;
    }
    let mut processor = StreamProcessor::new();
    processor.process(&mut [0.25; 960], true).unwrap();
    let allocation = processor.scratch.as_ptr();
    let capacity = processor.scratch.capacity();

    processor.process(&mut [0.25; 960], true).unwrap();
    processor.process(&mut [0.25; 480], true).unwrap();

    assert_eq!(processor.scratch.as_ptr(), allocation);
    assert_eq!(processor.scratch.capacity(), capacity);
}
