class CheenHubMicrophoneCapture extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const frameCount = options?.processorOptions?.frameCount ?? 480;
    this.frameCount = Math.max(128, Math.floor(frameCount));
    this.diagnosticsEnabled = Boolean(options?.processorOptions?.diagnosticsEnabled);
    this.samples = new Float32Array(this.frameCount);
    this.offset = 0;
    this.nextChunkFrame = 0;
    this.absoluteFrame = 0;
    this.profileWindowStartFrame = 0;
    this.profileProcessCount = 0;
    this.profileChunkCount = 0;
    this.profileInputEmptyCount = 0;
    this.profileFrames = 0;
    this.profileTotalElapsedUs = 0;
    this.profileClearElapsedUs = 0;
    this.profileCopyElapsedUs = 0;
    this.profilePostElapsedUs = 0;
    this.profileAllocateElapsedUs = 0;
    this.profileMaxProcessUs = 0;
    this.profileMaxChannels = 0;
  }

  process(inputs, outputs) {
    if (!this.diagnosticsEnabled) {
      this.clearOutputs(outputs);
      const input = inputs[0];
      if (!input || input.length === 0 || input[0].length === 0) {
        return true;
      }

      this.copyInput(input);
      return true;
    }

    const startedAtUs = this.nowUs();
    const clearStartedAtUs = this.nowUs();
    this.clearOutputs(outputs);
    this.profileClearElapsedUs += this.elapsedUs(clearStartedAtUs);

    const input = inputs[0];
    if (!input || input.length === 0 || input[0].length === 0) {
      this.profileInputEmptyCount += 1;
      this.recordProcessProfile(startedAtUs, 0, 0);
      return true;
    }

    const frames = input[0].length;
    const channels = input.length;
    const copyStartedAtUs = this.nowUs();
    this.copyInput(input);
    this.profileCopyElapsedUs += this.elapsedUs(copyStartedAtUs);
    this.recordProcessProfile(startedAtUs, frames, channels);

    return true;
  }

  copyInput(input) {
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
  }

  postChunk() {
    const chunk = this.samples;
    const timestampUs = Math.round((this.nextChunkFrame * 1_000_000) / sampleRate);
    const postStartedAtUs = this.diagnosticsEnabled ? this.nowUs() : 0;
    this.port.postMessage(
      {
        samples: chunk,
        timestampUs,
      },
      [chunk.buffer],
    );
    if (this.diagnosticsEnabled) {
      this.profilePostElapsedUs += this.elapsedUs(postStartedAtUs);
    }
    const allocateStartedAtUs = this.diagnosticsEnabled ? this.nowUs() : 0;
    this.samples = new Float32Array(this.frameCount);
    if (this.diagnosticsEnabled) {
      this.profileAllocateElapsedUs += this.elapsedUs(allocateStartedAtUs);
      this.profileChunkCount += 1;
    }
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

  recordProcessProfile(startedAtUs, frames, channels) {
    const elapsedUs = this.elapsedUs(startedAtUs);
    this.profileProcessCount += 1;
    this.profileFrames += frames;
    this.profileTotalElapsedUs += elapsedUs;
    this.profileMaxProcessUs = Math.max(this.profileMaxProcessUs, elapsedUs);
    this.profileMaxChannels = Math.max(this.profileMaxChannels, channels);
    const windowFrames = this.absoluteFrame - this.profileWindowStartFrame;
    if (windowFrames < sampleRate * 5) {
      return;
    }

    this.port.postMessage({
      kind: "profile",
      processCount: this.profileProcessCount,
      chunkCount: this.profileChunkCount,
      inputEmptyCount: this.profileInputEmptyCount,
      frames: this.profileFrames,
      windowMs: Math.round((windowFrames * 1000) / sampleRate),
      totalElapsedUs: Math.round(this.profileTotalElapsedUs),
      clearElapsedUs: Math.round(this.profileClearElapsedUs),
      copyElapsedUs: Math.round(this.profileCopyElapsedUs),
      postElapsedUs: Math.round(this.profilePostElapsedUs),
      allocateElapsedUs: Math.round(this.profileAllocateElapsedUs),
      maxProcessUs: Math.round(this.profileMaxProcessUs),
      maxChannels: this.profileMaxChannels,
    });
    this.resetProcessProfile();
  }

  resetProcessProfile() {
    this.profileWindowStartFrame = this.absoluteFrame;
    this.profileProcessCount = 0;
    this.profileChunkCount = 0;
    this.profileInputEmptyCount = 0;
    this.profileFrames = 0;
    this.profileTotalElapsedUs = 0;
    this.profileClearElapsedUs = 0;
    this.profileCopyElapsedUs = 0;
    this.profilePostElapsedUs = 0;
    this.profileAllocateElapsedUs = 0;
    this.profileMaxProcessUs = 0;
    this.profileMaxChannels = 0;
  }

  nowUs() {
    if (globalThis.performance?.now) {
      return globalThis.performance.now() * 1000;
    }
    if (globalThis.Date?.now) {
      return Date.now() * 1000;
    }
    return currentFrame * 1_000_000 / sampleRate;
  }

  elapsedUs(startedAtUs) {
    if (startedAtUs === 0) {
      return 0;
    }
    return this.nowUs() - startedAtUs;
  }
}

registerProcessor("cheenhub-microphone-capture", CheenHubMicrophoneCapture);
