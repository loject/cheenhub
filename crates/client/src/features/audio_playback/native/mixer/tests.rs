use super::*;

#[test]
fn mixer_sums_senders_and_clamps_output() {
    let mut mixer = MixerState {
        senders: HashMap::from([
            (
                "a".to_owned(),
                SenderMixerState {
                    samples: VecDeque::from(vec![0.75]),
                    gain: 1.0,
                    loop_samples: None,
                    loop_position: 0,
                    loop_gain: None,
                    fade_remaining_samples: None,
                },
            ),
            (
                "b".to_owned(),
                SenderMixerState {
                    samples: VecDeque::from(vec![0.75]),
                    gain: 1.0,
                    loop_samples: None,
                    loop_position: 0,
                    loop_gain: None,
                    fade_remaining_samples: None,
                },
            ),
        ]),
        output_gain: 1.0,
    };

    assert_eq!(mixer.next_sample(), 1.0);
}

#[test]
fn resampler_preserves_source_samples_at_equal_rate() {
    let mixer = new_mixer(1.0);
    queue_sender_samples(&mixer, "sender", vec![0.25, 0.5, 0.75], 1.0, 1);
    let mut mixer = mixer.lock().expect("mixer lock");
    let mut resampler = OutputResampler::new(48_000, 48_000);

    assert_eq!(resampler.next_sample(&mut mixer), 0.25);
    assert_eq!(resampler.next_sample(&mut mixer), 0.5);
    assert_eq!(resampler.next_sample(&mut mixer), 0.75);
}

#[test]
fn looped_sender_restarts_after_last_sample() {
    let mixer = new_mixer(1.0);
    queue_then_loop_sender_samples(&mixer, "signal", Vec::new(), vec![0.25, 0.5], 1.0, 1.0);
    let mut mixer = mixer.lock().expect("mixer lock");

    assert_eq!(mixer.next_sample(), 0.25);
    assert_eq!(mixer.next_sample(), 0.5);
    assert_eq!(mixer.next_sample(), 0.25);
}

#[test]
fn one_shot_finishes_before_loop_and_loop_fades_out() {
    let mixer = new_mixer(1.0);
    queue_then_loop_sender_samples(&mixer, "signal", vec![0.2, 0.4], vec![1.0], 1.0, 1.0);
    fade_out_sender(&mixer, "signal", 2);
    let mut state = mixer.lock().expect("mixer lock");
    assert_eq!(state.next_sample(), 0.2);
    assert_eq!(state.next_sample(), 0.4);
    assert_eq!(state.next_sample(), 1.0);
    assert_eq!(state.next_sample(), 0.5);
    assert_eq!(state.next_sample(), 0.0);
    assert!(!state.senders.contains_key("signal"));
}

#[test]
fn resampler_interpolates_when_output_rate_is_higher() {
    let mixer = new_mixer(1.0);
    queue_sender_samples(&mixer, "sender", vec![0.0, 1.0], 1.0, 1);
    let mut mixer = mixer.lock().expect("mixer lock");
    let mut resampler = OutputResampler::new(48_000, 96_000);

    assert_eq!(resampler.next_sample(&mut mixer), 0.0);
    assert_eq!(resampler.next_sample(&mut mixer), 0.5);
    assert_eq!(resampler.next_sample(&mut mixer), 1.0);
}
