class CheenHubMicrophoneCapture extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const frameCount = options?.processorOptions?.frameCount ?? 480;
    this.frameCount = Math.max(128, Math.floor(frameCount));
    this.samples = new Float32Array(this.frameCount);
    this.offset = 0;
    this.nextChunkFrame = 0;
    this.absoluteFrame = 0;
  }

  process(inputs, outputs) {
    this.clearOutputs(outputs);

    const input = inputs[0];
    if (!input || input.length === 0 || input[0].length === 0) {
      return true;
    }

    const frames = input[0].length;
    const channels = input.length;
    for (let frame = 0; frame < frames; frame += 1) {
      if (this.offset === 0) {
        this.nextChunkFrame = this.absoluteFrame;
      }

      let sample = 0;
      for (let channel = 0; channel < channels; channel += 1) {
        sample += input[channel][frame] ?? 0;
      }
      this.samples[this.offset] = sample / channels;
      this.offset += 1;
      this.absoluteFrame += 1;

      if (this.offset === this.frameCount) {
        this.postChunk();
      }
    }

    return true;
  }

  postChunk() {
    const chunk = this.samples;
    const timestampUs = Math.round((this.nextChunkFrame * 1_000_000) / sampleRate);
    this.port.postMessage(
      {
        samples: chunk,
        timestampUs,
      },
      [chunk.buffer],
    );
    this.samples = new Float32Array(this.frameCount);
    this.offset = 0;
  }

  clearOutputs(outputs) {
    const output = outputs[0];
    if (!output) {
      return;
    }
    for (const channel of output) {
      channel.fill(0);
    }
  }
}

registerProcessor("cheenhub-microphone-capture", CheenHubMicrophoneCapture);
