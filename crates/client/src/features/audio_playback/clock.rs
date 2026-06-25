//! Тактовый генератор воспроизведения на основе AudioWorklet.
//!
//! Раньше цикл слива jitter-буфера пейсился `setTimeout` на главном потоке, который
//! браузер троттлит до ~1 c в неактивной вкладке — отсюда заикания входящего звука,
//! когда окно приложения не в фокусе. Этот ворклет тикает на аудио-потоке рендеринга,
//! который не троттлится, и по каждому тику главный поток сливает буферы.

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioContext, AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext, Worklet};

const PLAYBACK_CLOCK_WORKLET_URL: &str = "/audio/playback-clock-worklet.js?v=1";
const PLAYBACK_CLOCK_PROCESSOR_NAME: &str = "cheenhub-playback-clock";

/// Загружает модуль ворклета тактового генератора воспроизведения.
pub(super) async fn load_playback_clock_module(context: &AudioContext) -> Result<(), JsValue> {
    let worklet = context.audio_worklet()?;
    let worklet = worklet.unchecked_ref::<Worklet>();
    let promise = worklet.add_module(PLAYBACK_CLOCK_WORKLET_URL)?;
    JsFuture::from(promise).await?;
    Ok(())
}

/// Создаёт узел-генератор, подключённый к выходу контекста, чтобы граф опрашивал его.
pub(super) fn create_playback_clock_node(
    context: &AudioContext,
) -> Result<AudioWorkletNode, JsValue> {
    let options = AudioWorkletNodeOptions::new();
    options.set_number_of_inputs(0);
    options.set_number_of_outputs(1);

    let base = context.unchecked_ref::<BaseAudioContext>();
    let node = AudioWorkletNode::new_with_options(base, PLAYBACK_CLOCK_PROCESSOR_NAME, &options)?;
    // Узел должен быть в графе, чтобы его process() регулярно опрашивался; он выдаёт тишину.
    node.connect_with_audio_node(context.destination().as_ref())?;
    Ok(node)
}
