// Тактовый генератор воспроизведения.
//
// process() AudioWorkletProcessor выполняется на аудио-потоке рендеринга, который
// браузер НЕ троттлит в неактивной вкладке (в отличие от setTimeout / requestAnimationFrame).
// Каждые ~tickFrames сэмплов воркл шлёт в главный поток сообщение-тик; по нему слой
// воспроизведения сливает jitter-буферы и кормит декодер. Узел выдаёт тишину и
// подключён к destination только для того, чтобы граф регулярно опрашивал его process().
class CheenHubPlaybackClock extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const requested = options && options.processorOptions
      ? options.processorOptions.tickFrames
      : undefined;
    const fallback = Math.round(sampleRate / 100); // ~10 мс при текущей частоте дискретизации
    this.tickFrames = Math.max(128, Math.floor(requested != null ? requested : fallback));
    this.elapsed = 0;
  }

  process(_inputs, outputs) {
    const output = outputs[0];
    const quantum = output && output[0] ? output[0].length : 128;
    if (output) {
      for (const channel of output) {
        channel.fill(0);
      }
    }

    this.elapsed += quantum;
    if (this.elapsed >= this.tickFrames) {
      this.elapsed = 0;
      this.port.postMessage(0);
    }

    return true;
  }
}

registerProcessor("cheenhub-playback-clock", CheenHubPlaybackClock);
