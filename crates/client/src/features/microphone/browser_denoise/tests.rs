use super::super::denoise::RuntimeSwitch;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::*;

fn callbacks(warnings: Rc<Cell<u32>>) -> MicrophoneCallbacks {
    MicrophoneCallbacks {
        on_frame: Rc::new(|_| {}),
        on_level: Rc::new(|_| {}),
        on_error: Rc::new(|_| panic!("DSP failure must not stop capture")),
        on_warning: Rc::new(move |_| warnings.set(warnings.get() + 1)),
        uplink: None,
    }
}

#[test]
fn live_rnnoise_switch_keeps_unprocessed_pcm_when_disabled() {
    let enabled = Arc::new(RuntimeSwitch::new(false));
    let mut denoiser = BrowserDenoiser::new(MicrophoneConfig {
        denoise_enabled_live: Some(enabled.clone()),
        ..MicrophoneConfig::default()
    });
    let warnings = Rc::new(Cell::new(0));
    let callbacks = callbacks(warnings.clone());
    let mut samples = [0.25; 480];

    assert_eq!(denoiser.process(&mut samples, 48_000, &callbacks), None);
    assert_eq!(samples, [0.25; 480]);
    enabled.store(true, Ordering::Relaxed);
    assert_eq!(
        denoiser.process(&mut samples, 48_000, &callbacks).is_some(),
        super::super::denoise::available()
    );
    enabled.store(false, Ordering::Relaxed);
    samples.fill(0.25);
    assert_eq!(denoiser.process(&mut samples, 48_000, &callbacks), None);
    assert_eq!(samples, [0.25; 480]);
    assert_eq!(warnings.get(), 0);
}

#[test]
fn dsp_failure_disables_rnnoise_preserves_pcm_and_warns_once() {
    let enabled = Arc::new(RuntimeSwitch::new(true));
    let mut denoiser = BrowserDenoiser::new(MicrophoneConfig {
        denoise_enabled_live: Some(enabled.clone()),
        ..MicrophoneConfig::default()
    });
    let warnings = Rc::new(Cell::new(0));
    let callbacks = callbacks(warnings.clone());
    let mut samples = [0.25; 481];

    assert_eq!(denoiser.process(&mut samples, 48_000, &callbacks), None);
    let available = super::super::denoise::available();
    assert_eq!(enabled.load(Ordering::Relaxed), !available);
    assert_eq!(samples, [0.25; 481]);
    assert_eq!(warnings.get(), u32::from(available));
    assert_eq!(denoiser.process(&mut samples, 48_000, &callbacks), None);
    assert_eq!(warnings.get(), u32::from(available));
}

#[test]
fn browser_main_reenables_rnnoise_after_failure_on_same_capture() {
    let runtime = Arc::new(RuntimeSwitch::new(true));
    let mut denoiser = BrowserDenoiser::new(MicrophoneConfig {
        denoise_enabled_live: Some(runtime.clone()),
        ..MicrophoneConfig::default()
    });
    let warnings = Rc::new(Cell::new(0));
    let callbacks = callbacks(warnings.clone());
    denoiser.process(&mut [0.25; 481], 48_000, &callbacks);
    if !super::super::denoise::available() {
        return;
    }
    assert!(runtime.failed());

    runtime.store(true, Ordering::Relaxed);
    let probability = denoiser.process(&mut [0.25; 480], 48_000, &callbacks);

    assert!(probability.is_some());
    assert!(!runtime.failed());
    assert!(runtime.load(Ordering::Relaxed));
    assert_eq!(warnings.get(), 1);
}
