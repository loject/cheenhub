class CheenHubScreenAudio extends AudioWorkletProcessor {
  constructor() {
    super();
    this.frames = 960;
    this.samples = new Float32Array(this.frames * 2);
    this.offset = 0;
    this.inFlight = 0;
    this.closed = false;
    this.port.onmessage = ({data}) => {
      if (data === "consumed") this.inFlight = Math.max(0, this.inFlight - 1);
      if (data?.kind === "stop") this.closed = true;
    };
  }
  process(inputs, outputs) {
    if (this.closed) return false;
    for (const output of outputs) for (const channel of output) channel.fill(0);
    const input = inputs[0];
    if (!input?.[0]?.length) return true;
    for (let i = 0; i < input[0].length; i++) {
      if (this.offset === 0) this.startFrame = currentFrame + i;
      this.samples[this.offset] = input[0][i];
      this.samples[this.frames + this.offset] = (input[1] ?? input[0])[i];
      if (++this.offset === this.frames) {
        if (this.inFlight < 2) {
          this.port.postMessage({samples: this.samples, timestampUs: this.startFrame * 1e6 / sampleRate}, [this.samples.buffer]);
          this.inFlight++;
          this.samples = new Float32Array(this.frames * 2);
        }
        this.offset = 0;
      }
    }
    return true;
  }
}
registerProcessor("cheenhub-screen-audio", CheenHubScreenAudio);
