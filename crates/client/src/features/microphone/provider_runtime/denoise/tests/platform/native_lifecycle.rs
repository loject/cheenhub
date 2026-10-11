use super::super::super::*;
use crate::features::microphone::{
    backend::{MicrophoneBackend, MicrophoneCallbacks, MicrophoneConfig, MicrophoneSession},
    denoise::RuntimeSwitch,
    provider::{ActiveCapture, MicrophoneCommand, MicrophoneHandle},
};
use futures_channel::mpsc;
use futures_util::future::LocalBoxFuture;
use futures_util::{
    FutureExt,
    future::{pending, ready},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

thread_local! {
    static PROBE: RefCell<Option<Probe>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct Probe {
    runtime: MicrophoneRuntime,
    commands: mpsc::UnboundedSender<MicrophoneCommand>,
    stopped: Rc<Cell<bool>>,
    cleaned: Rc<Cell<u32>>,
    result: Rc<Cell<u8>>,
}

struct Session {
    stopped: Rc<Cell<bool>>,
    cleaned: Rc<Cell<u32>>,
    result: Rc<Cell<u8>>,
}
impl MicrophoneSession for Session {
    fn set_input_gain(&self, _: f32) -> Result<(), MicrophoneError> {
        Ok(())
    }
    fn stop_immediately(&self) {
        self.stopped.set(true);
    }
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        if self.result.get() == 4 {
            self.cleaned.set(self.cleaned.get() + 1);
            return pending().boxed_local();
        }
        let cleaned = self.cleaned.clone();
        async move {
            cleaned.set(cleaned.get() + 1);
            Ok(())
        }
        .boxed_local()
    }
    fn set_bitrate_bps(&self, _: u32) -> LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        ready(Ok(())).boxed_local()
    }
    fn set_denoise_mode(
        &self,
        mode: DenoiseMode,
        runtime: Arc<RuntimeSwitch>,
    ) -> LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        match self.result.get() {
            0 => pending().boxed_local(),
            1 => ready(Err(MicrophoneError::new("constraints rejected"))).boxed_local(),
            3 | 4 => ready(Err(MicrophoneError::invalidated("rollback failed"))).boxed_local(),
            _ => async move {
                runtime.store(mode == DenoiseMode::Rnnoise, Ordering::Relaxed);
                Ok(())
            }
            .boxed_local(),
        }
    }
}
struct Backend;
impl MicrophoneBackend for Backend {
    fn start(
        &self,
        _: MicrophoneConfig,
        _: MicrophoneCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn MicrophoneSession>, MicrophoneError>> {
        panic!("test must preserve existing voice session")
    }
}

fn app() -> Element {
    let focused = use_signal(|| true);
    use_context_provider(move || {
        crate::features::application_focus::ApplicationFocusContext::new(focused)
    });
    rsx! { crate::features::toast::ToastProvider { RuntimeProbe {} } }
}

#[component]
fn RuntimeProbe() -> Element {
    let toast = use_context::<crate::features::toast::ToastHandle>();
    use_hook(move || {
        let handle = MicrophoneHandle::for_test();
        let stopped = Rc::new(Cell::new(false));
        let cleaned = Rc::new(Cell::new(0));
        let result = Rc::new(Cell::new(0));
        let session: Rc<dyn MicrophoneSession> = Rc::new(Session {
            stopped: stopped.clone(),
            cleaned: cleaned.clone(),
            result: result.clone(),
        });
        let runtime = MicrophoneRuntime {
            backend: Rc::new(Backend),
            status: Signal::new(MicrophoneStatus::Live),
            session: Signal::new(Some(session)),
            generation: handle.generation,
            level: handle.level,
            level_active: handle.level_active,
            selected_input_device_id: handle.selected_input_device_id,
            input_volume_percent: handle.input_volume_percent,
            recording_generation: handle.recording_generation,
            push_to_talk_key: handle.push_to_talk_key,
            activation_mode: handle.activation_mode,
            activation_runtime: handle.activation_runtime,
            vad_threshold_percent: handle.vad_threshold_percent,
            neural_vad_threshold_percent: handle.neural_vad_threshold_percent,
            denoise_mode: handle.denoise_mode,
            denoise_pending: handle.denoise_pending,
            denoise_failed: handle.denoise_failed,
            denoise_runtime: handle.denoise_runtime,
            denoise_timeout: || pending().boxed_local(),
            denoise_cancel: Rc::new(RefCell::new(None)),
            active_capture: Signal::new(ActiveCapture::Voice),
            active_on_frame: handle.active_on_frame,
            active_uplink: handle.active_uplink,
            target_bitrate_bps: handle.target_bitrate_bps,
            toast,
        };
        let (commands, receiver) = mpsc::unbounded();
        PROBE.with_borrow_mut(|probe| {
            *probe = Some(Probe {
                runtime: runtime.clone(),
                commands,
                stopped,
                cleaned,
                result,
            })
        });
        spawn(
            crate::features::microphone::provider_runtime::run_microphone_runtime(
                receiver, runtime,
            ),
        );
    });
    rsx! {}
}

fn pump(dom: &mut VirtualDom) {
    let _ = dom.wait_for_work().now_or_never();
    dom.render_immediate_to_vec();
}

#[tokio::test]
async fn stop_is_processed_while_constraints_are_pending_and_old_update_cannot_commit() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let probe = PROBE.with_borrow(|probe| probe.clone().unwrap());
    dom.in_scope(ScopeId::ROOT, || {
        probe
            .commands
            .unbounded_send(MicrophoneCommand::SetDenoise {
                mode: DenoiseMode::Rnnoise,
                operation_generation: 0,
            })
            .unwrap()
    });
    pump(&mut dom);
    assert!(*probe.runtime.denoise_pending.peek());

    dom.in_scope(ScopeId::ROOT, || {
        let mut generation = probe.runtime.generation;
        generation.set(1);
        probe
            .commands
            .unbounded_send(MicrophoneCommand::Stop {
                operation_generation: 1,
            })
            .unwrap();
    });
    pump(&mut dom);

    assert!(probe.stopped.get());
    assert!(!*probe.runtime.denoise_pending.peek());
    assert_eq!(*probe.runtime.denoise_mode.peek(), DenoiseMode::Off);
    assert!(!probe.runtime.denoise_runtime.load(Ordering::Relaxed));
}

#[tokio::test]
async fn rejected_live_change_keeps_preference_and_dsp_then_retry_commits() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let probe = PROBE.with_borrow(|probe| probe.clone().unwrap());
    probe.result.set(1);
    let saved = Rc::new(RefCell::new(Vec::new()));
    let record = saved.clone();
    dom.in_scope(ScopeId::ROOT, || {
        apply_with_persistence(probe.runtime.clone(), DenoiseMode::Rnnoise, move |mode| {
            record.borrow_mut().push(mode)
        })
    });
    pump(&mut dom);

    assert_eq!(*probe.runtime.denoise_mode.peek(), DenoiseMode::Off);
    assert!(!probe.runtime.denoise_runtime.load(Ordering::Relaxed));
    assert!(!probe.stopped.get());
    assert!(!*probe.runtime.denoise_pending.peek());

    probe.result.set(2);
    assert!(saved.borrow().is_empty());
    let record = saved.clone();
    dom.in_scope(ScopeId::ROOT, || {
        apply_with_persistence(probe.runtime.clone(), DenoiseMode::Rnnoise, move |mode| {
            record.borrow_mut().push(mode)
        })
    });
    pump(&mut dom);

    assert_eq!(*saved.borrow(), [DenoiseMode::Rnnoise]);
    assert_eq!(*probe.runtime.denoise_mode.peek(), DenoiseMode::Rnnoise);
    assert!(probe.runtime.denoise_runtime.load(Ordering::Relaxed));
    assert!(!probe.stopped.get());
    assert!(!*probe.runtime.denoise_pending.peek());
}

#[tokio::test]
async fn timeout_revokes_voice_session_and_preserves_saved_selection() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let probe = PROBE.with_borrow(|probe| probe.clone().unwrap());
    let mut runtime = probe.runtime.clone();
    runtime.denoise_timeout = || ready(()).boxed_local();
    dom.in_scope(ScopeId::ROOT, || {
        apply_with_persistence(runtime, DenoiseMode::Rnnoise, |_| {
            panic!("timeout must not save a selection")
        })
    });

    pump(&mut dom);

    pump(&mut dom);
    assert!(probe.stopped.get());
    assert_eq!(probe.cleaned.get(), 1);
    assert_eq!(*probe.runtime.denoise_mode.peek(), DenoiseMode::Off);
    assert_eq!(*probe.runtime.generation.peek(), 1);
    assert!(!*probe.runtime.denoise_pending.peek());
    assert!(matches!(
        *probe.runtime.status.peek(),
        MicrophoneStatus::Error(_)
    ));
}

#[tokio::test]
async fn dsp_failure_updates_runtime_ui_and_same_mode_retry_restores_processing() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let mut probe = PROBE.with_borrow(|probe| probe.clone().unwrap());
    dom.in_scope(ScopeId::ROOT, || {
        probe.runtime.denoise_mode.set(DenoiseMode::Rnnoise);
        probe.runtime.denoise_runtime.store(true, Ordering::Relaxed);
        let callbacks = crate::features::microphone::provider_runtime::microphone_callbacks(
            Rc::new(|_| {}),
            probe.runtime.clone(),
            0,
            None,
        );
        assert!(
            probe
                .runtime
                .denoise_runtime
                .fail(probe.runtime.denoise_runtime.snapshot())
        );
        (callbacks.on_warning)("RNNoise failed");
    });
    pump(&mut dom);

    assert!(*probe.runtime.denoise_failed.peek());
    assert_eq!(*probe.runtime.denoise_mode.peek(), DenoiseMode::Rnnoise);
    assert_eq!(*probe.runtime.status.peek(), MicrophoneStatus::Live);
    assert!(!probe.runtime.denoise_runtime.load(Ordering::Relaxed));

    probe.result.set(2);
    dom.in_scope(ScopeId::ROOT, || {
        apply_with_persistence(probe.runtime.clone(), DenoiseMode::Rnnoise, |_| {})
    });
    pump(&mut dom);

    assert!(!*probe.runtime.denoise_failed.peek());
    assert!(probe.runtime.denoise_runtime.load(Ordering::Relaxed));
    assert!(!probe.stopped.get());
}

#[tokio::test]
async fn denoise_command_from_old_voice_generation_is_ignored() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let mut probe = PROBE.with_borrow(|probe| probe.clone().unwrap());
    probe.result.set(2);
    dom.in_scope(ScopeId::ROOT, || {
        probe.runtime.generation.set(1);
        probe
            .commands
            .unbounded_send(MicrophoneCommand::SetDenoise {
                mode: DenoiseMode::Rnnoise,
                operation_generation: 0,
            })
            .unwrap();
    });

    pump(&mut dom);

    assert_eq!(*probe.runtime.denoise_mode.peek(), DenoiseMode::Off);
    assert!(!probe.runtime.denoise_runtime.load(Ordering::Relaxed));
    assert!(!probe.stopped.get());
}

#[tokio::test]
async fn invalidated_change_cleans_up_once_without_mutating_new_session() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let probe = PROBE.with_borrow(|probe| probe.clone().unwrap());
    probe.result.set(3);
    dom.in_scope(ScopeId::ROOT, || {
        apply_with_persistence(probe.runtime.clone(), DenoiseMode::Browser, |_| {
            panic!("must not persist")
        });
    });
    pump(&mut dom);
    pump(&mut dom);

    assert!(probe.stopped.get());
    assert_eq!(probe.cleaned.get(), 1);
    assert!(probe.runtime.session.peek().is_none());
    assert_eq!(*probe.runtime.generation.peek(), 1);
    assert!(!*probe.runtime.denoise_pending.peek());
}

#[tokio::test]
async fn pending_cleanup_does_not_block_next_denoise_command() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let probe = PROBE.with_borrow(|probe| probe.clone().unwrap());
    probe.result.set(4);
    dom.in_scope(ScopeId::ROOT, || {
        apply_with_persistence(probe.runtime.clone(), DenoiseMode::Browser, |_| {
            panic!("must not persist")
        });
    });
    pump(&mut dom);
    pump(&mut dom);
    assert_eq!(probe.cleaned.get(), 1);

    let saved = Rc::new(Cell::new(false));
    let record = saved.clone();
    dom.in_scope(ScopeId::ROOT, || {
        apply_with_persistence(probe.runtime.clone(), DenoiseMode::Off, move |_| {
            record.set(true)
        });
    });
    pump(&mut dom);

    assert!(saved.get());
    assert_eq!(probe.cleaned.get(), 1);
    assert_eq!(*probe.runtime.generation.peek(), 1);
    assert!(!*probe.runtime.denoise_pending.peek());
}
