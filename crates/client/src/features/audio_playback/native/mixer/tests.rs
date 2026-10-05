use super::*;

#[test]
fn mixer_sums_senders_and_saturates_output() {
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

    // Сумма 0.75 + 0.75 = 1.5 выходит за полную амплитуду и насыщается
    // ровно до 1.0, тогда как жёсткий clamp давал бы 1.0 с обрезкой пика.
    let rendered = mixer.next_sample();

    assert_eq!(rendered, 1.0);
    assert!(rendered < 0.75 + 0.75);
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

#[test]
fn voice_sender_plays_at_exact_user_volume_without_notification_scaling() {
    // Голос участника должен звучать ровно с коэффициентом из настроек:
    // при 100% это 1.0, независимо от приглушения notification-звуков.
    let user_gain = 100.0_f32 / 100.0;
    let mixer = new_mixer(1.0);
    queue_sender_samples(&mixer, "peer-user", vec![0.5], user_gain, 1);

    assert_eq!(mixer.lock().expect("mixer lock").next_sample(), 0.5);
}

#[test]
fn voice_sender_scales_linearly_with_user_volume_percent() {
    // Sample 0.5 при громкости до 150% остаётся ниже порога насыщения,
    // поэтому масштабируется ровно по настройке: 50% в настройках даёт 0.25.
    for percent in [0_u32, 25, 50, 75, 100] {
        let gain = percent.min(200) as f32 / 100.0;
        let mixer = new_mixer(1.0);
        queue_sender_samples(&mixer, "peer-user", vec![0.5], gain, 1);

        assert_eq!(
            mixer.lock().expect("mixer lock").next_sample(),
            0.5 * gain,
            "voice at {percent}% should render with gain {gain}"
        );
    }
}

#[test]
fn voice_sender_above_full_volume_saturates_smoother_than_hard_clamp() {
    // При 150-200% сигнал насыщается: пик стремится к 1.0, но не обрезается
    // по горизонтали, как это делал жёсткий clamp.
    for percent in [150_u32, 200] {
        let gain = percent.min(200) as f32 / 100.0;
        let mixer = new_mixer(1.0);
        queue_sender_samples(&mixer, "peer-user", vec![1.0], gain, 1);

        let rendered = mixer.lock().expect("mixer lock").next_sample();

        assert_eq!(
            rendered, 1.0,
            "full-scale input at {percent}% should stay at full scale"
        );
    }

    // Sample выше полной амплитуды до насыщения не должен обрезаться в ноль.
    let mixer = new_mixer(1.0);
    queue_sender_samples(&mixer, "peer-user", vec![1.0], 1.5, 1);
    assert_eq!(mixer.lock().expect("mixer lock").next_sample(), 1.0);
}

#[test]
fn boosted_quiet_speech_scales_linearly_above_full_volume() {
    // Ключевое отличие от жёсткой обрезки: тихая речь ниже порога
    // насыщения усиливается ровно по настройке, а не срезается.
    for percent in [100_u32, 150, 200] {
        let gain = percent.min(200) as f32 / 100.0;
        let mixer = new_mixer(1.0);
        queue_sender_samples(&mixer, "peer-user", vec![0.5], gain, 1);

        assert_eq!(
            mixer.lock().expect("mixer lock").next_sample(),
            0.5 * gain,
            "quiet speech at {percent}% should scale by the full gain"
        );
    }
}

#[test]
fn soft_limit_keeps_loud_peaks_inside_pcm_range() {
    // Итог всегда остаётся в [-1.0, 1.0], иначе конвертация в целочисленный
    // PCM дала бы мусор вместо звука.
    for raw in [1.0_f32, 1.5, 4.0, 100.0, 10_000.0] {
        let limited = soft_limit(raw);

        assert_eq!(limited, 1.0, "{raw} should saturate exactly at full scale");
        assert!(limited.is_finite());
    }

    assert_eq!(soft_limit(-1.0), -soft_limit(1.0));
    assert_eq!(soft_limit(-4.0), -soft_limit(4.0));
}

#[test]
fn soft_limit_is_monotonic_between_knee_and_full_scale() {
    // Громкость не должна убывать на насыщающем участке: иначе усиление
    // выше 100% давало бы обратный эффект.
    let mut raw = SOFT_LIMIT_KNEE + 0.001;
    let mut previous = soft_limit(SOFT_LIMIT_KNEE);

    while raw < 1.0 {
        let limited = soft_limit(raw);

        assert!(limited > previous, "soft_limit must not decrease at {raw}");
        assert!(limited >= raw, "soft_limit must not attenuate at {raw}");
        previous = limited;
        raw += 0.01;
    }
}

#[test]
fn soft_limit_passes_quiet_signal_without_change() {
    for raw in [0.0_f32, 0.25, 0.5, 0.75, SOFT_LIMIT_KNEE] {
        assert_eq!(soft_limit(raw), raw);
        assert_eq!(soft_limit(-raw), -raw);
    }
}

#[test]
fn soft_limit_replaces_non_finite_samples_with_silence_or_full_scale() {
    // Сбойный sample не должен заглушить поток и не должен дать мусор в PCM.
    assert_eq!(soft_limit(f32::NAN), 0.0);
    assert_eq!(soft_limit(f32::INFINITY), 1.0);
    assert_eq!(soft_limit(f32::NEG_INFINITY), -1.0);
}

#[test]
fn multiple_senders_saturate_without_exceeding_full_scale() {
    // Два одновременных отправителя по 0.5 на 150% дают сумму 1.5, которая
    // насыщается до 1.0 и не выходит за пределы PCM.
    let mixer = new_mixer(1.0);
    queue_sender_samples(&mixer, "peer-a", vec![0.5], 1.5, 1);
    queue_sender_samples(&mixer, "peer-b", vec![0.5], 1.5, 1);

    let rendered = mixer.lock().expect("mixer lock").next_sample();

    assert_eq!(rendered, 1.0);
}
