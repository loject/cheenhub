const TRANSPORT_WEBTRANSPORT = "webtransport";
const TRANSPORT_WEBSOCKET = "websocket";
const MAX_PCM_AGE_MS = 180;
const MAX_ENCODER_QUEUE_FRAMES = 4;
const MAX_WEBSOCKET_BUFFERED_BYTES = 64 * 1024;
const WARNING_INTERVAL_MS = 5_000;
const LEVEL_INTERVAL_MS = 50;

let active = null;

self.onmessage = (event) => {
  const message = event.data || {};
  if (message.kind === "start") {
    void start(message).catch((error) => post("error", { message: errorMessage(error) }));
  } else if (message.kind === "set-bitrate") {
    setBitrate(message.bitrateBps);
  } else if (message.kind === "stop") {
    void stopActive();
  }
};

async function start(config) {
  await stopActive();
  validateStartConfig(config);
  post("status", { message: "starting microphone uplink worker" });
  let transport = null;
  try {
    const wasm = await import(config.wasmBindgenUrl);
    await wasm.default(config.wasmUrl);
    if (wasm.microphone_worker_abi_version() < 3) {
      throw new Error("microphone worker wasm ABI is too old");
    }

    const processor = new wasm.MicrophoneWorkerProcessor(
      config.sampleRateHz,
      config.channels,
      config.activationMode,
      config.vadThreshold,
      config.vadActivationDelayUs,
      config.vadReleaseDelayUs,
      config.inputGain,
      config.roomId,
    );
    transport = await connectRealtime(config, wasm);
    const encoder = createEncoder(config, processor, transport);
    const startedWallMs = Date.now();
    active = {
      port: config.workletPort,
      encoder,
      processor,
      transport,
      sampleRateHz: config.sampleRateHz,
      bitrateBps: config.bitrateBps,
      startedWallMs,
      lastChunkWallMs: startedWallMs,
      lastTimestampUs: null,
      lastLevelMs: 0,
      lastLevelActive: false,
      sendPending: false,
      droppedPcm: 0,
      droppedEncoded: 0,
      lastWarningMs: 0,
      closed: false,
    };
    active.port.onmessage = (event) => handleWorkletMessage(event.data, active);
    active.port.start();
    post("ready", { transport: transport.kind });
    post("status", { message: "microphone uplink worker started", transport: transport.kind });
  } catch (error) {
    config.workletPort.close();
    if (transport) await transport.close();
    throw error;
  }
}

async function stopActive() {
  const current = active;
  active = null;
  if (!current || current.closed) {
    return;
  }
  current.closed = true;
  current.port.onmessage = null;
  current.port.close();
  try {
    current.encoder.reset();
    current.encoder.close();
  } catch (error) {
    post("warning", { message: "failed to close microphone worker encoder", detail: errorMessage(error) });
  }
  await current.transport.close();
  post("status", { message: "microphone uplink worker stopped" });
}

function setBitrate(value) {
  const current = active;
  const bitrateBps = Math.max(6_000, Math.floor(Number(value) || 0));
  if (!current || bitrateBps === current.bitrateBps) {
    return;
  }
  current.bitrateBps = bitrateBps;
  current.encoder.configure(encoderConfig(current.sampleRateHz, 1, bitrateBps));
}

function createEncoder(config, processor, transport) {
  const encoder = new AudioEncoder({
    output: (chunk) => handleEncodedChunk(chunk, processor, transport),
    error: (error) => failActive(`microphone worker encoder failed: ${errorMessage(error)}`),
  });
  encoder.configure(encoderConfig(config.sampleRateHz, config.channels, config.bitrateBps));
  return encoder;
}

function encoderConfig(sampleRateHz, channels, bitrateBps) {
  return {
    codec: "opus",
    sampleRate: sampleRateHz,
    numberOfChannels: channels,
    bitrate: bitrateBps,
    opus: {
      useinbandfec: true,
      usedtx: true,
      application: "voip",
      frameDuration: 20_000,
    },
  };
}

function handleWorkletMessage(data, current) {
  if (current.closed || data?.kind === "profile") {
    return;
  }
  try {
    const samples = data?.samples;
    if (!(samples instanceof Float32Array) || samples.length === 0) {
      return;
    }
    const timestampUs = Math.max(0, Math.floor(Number(data.timestampUs) || 0));
    const nowMs = Date.now();
    if (shouldRebaseAfterCapturePause(current, timestampUs, nowMs)) {
      current.startedWallMs = nowMs - timestampUs / 1000;
    }
    current.lastChunkWallMs = nowMs;
    current.lastTimestampUs = timestampUs;
    const pcmAgeMs = nowMs - (current.startedWallMs + timestampUs / 1000);
    if (pcmAgeMs > MAX_PCM_AGE_MS || current.encoder.encodeQueueSize >= MAX_ENCODER_QUEUE_FRAMES) {
      current.droppedPcm += 1;
      warnAboutDrops(current, pcmAgeMs);
      return;
    }

    const processed = current.processor.process_pcm(samples, timestampUs);
    emitLevel(current, processed, nowMs);
    if (!processed.active || !(processed.samples instanceof Float32Array)) {
      return;
    }
    const audio = new AudioData({
      format: "f32-planar",
      sampleRate: current.sampleRateHz,
      numberOfFrames: processed.samples.length,
      numberOfChannels: 1,
      timestamp: processed.timestampUs,
      data: processed.samples,
    });
    current.encoder.encode(audio);
    audio.close();
  } catch (error) {
    post("warning", { message: "failed to process microphone worker chunk", detail: errorMessage(error) });
  }
}

function shouldRebaseAfterCapturePause(current, timestampUs, nowMs) {
  if (current.lastTimestampUs === null) {
    return false;
  }
  const wallGapMs = nowMs - current.lastChunkWallMs;
  const audioGapMs = (timestampUs - current.lastTimestampUs) / 1000;
  return wallGapMs > 500 && audioGapMs < 100;
}

function emitLevel(current, processed, nowMs) {
  if (
    processed.active === current.lastLevelActive &&
    nowMs - current.lastLevelMs < LEVEL_INTERVAL_MS
  ) {
    return;
  }
  current.lastLevelMs = nowMs;
  current.lastLevelActive = processed.active;
  post("level", {
    rms: processed.rms,
    active: processed.active,
    threshold: processed.threshold,
    timestampUs: processed.timestampUs,
  });
}

function handleEncodedChunk(chunk, processor, transport) {
  const current = active;
  if (!current || current.closed) {
    return;
  }
  if (current.sendPending || !transport.canSend()) {
    current.droppedEncoded += 1;
    warnAboutDrops(current, 0);
    return;
  }
  try {
    const payload = new Uint8Array(chunk.byteLength);
    chunk.copyTo(payload);
    const datagram = processor.voice_datagram(
      payload,
      Math.max(0, Number(chunk.timestamp) || 0),
      Math.max(0, Number(chunk.duration) || 0),
    );
    current.sendPending = true;
    void transport.send(datagram).catch((error) => {
      failActive("microphone worker media send failed", errorMessage(error));
    }).finally(() => {
      if (active === current) {
        current.sendPending = false;
      }
    });
  } catch (error) {
    post("warning", { message: "failed to encode microphone media datagram", detail: errorMessage(error) });
  }
}

function warnAboutDrops(current, pcmAgeMs) {
  const nowMs = Date.now();
  if (current.lastWarningMs && nowMs - current.lastWarningMs < WARNING_INTERVAL_MS) {
    return;
  }
  current.lastWarningMs = nowMs;
  post("warning", {
    message: "microphone worker dropped stale uplink audio",
    droppedPcm: current.droppedPcm,
    droppedEncoded: current.droppedEncoded,
    pcmAgeMs: Math.max(0, Math.round(pcmAgeMs)),
    encoderQueueSize: current.encoder.encodeQueueSize,
  });
  current.droppedPcm = 0;
  current.droppedEncoded = 0;
}

async function connectRealtime(config, wasm) {
  try {
    return await connectWebTransport(config, wasm);
  } catch (webtransportError) {
    post("warning", {
      message: "microphone worker WebTransport failed; trying WebSocket fallback",
      detail: errorMessage(webtransportError),
    });
    return connectWebSocket(config, wasm);
  }
}

async function connectWebTransport(config, wasm) {
  const options = {};
  if (config.realtimeCertSha256) {
    options.serverCertificateHashes = [{ algorithm: "sha-256", value: hexToBytes(config.realtimeCertSha256) }];
  }
  const session = new WebTransport(config.realtimeUrl, options);
  await session.ready;
  const control = await session.createBidirectionalStream();
  const writer = control.writable.getWriter();
  const frameReader = createFrameReader(control.readable.getReader());
  await writer.write(wasm.microphone_worker_authenticate_webtransport_frame(config.accessToken));
  verifyEnvelope(await frameReader.readFrame(), "control", "authenticated");
  await writer.write(wasm.microphone_worker_bind_uplink_webtransport_frame(config.uplinkGrant));
  verifyEnvelope(await frameReader.readFrame(), "voice_chat", "microphone_uplink_bound");
  const datagramWriter = session.datagrams.writable.getWriter();
  return {
    kind: TRANSPORT_WEBTRANSPORT,
    canSend: () => (datagramWriter.desiredSize ?? 1) > 0,
    send: (bytes) => datagramWriter.write(bytes),
    close: async () => {
      try { await datagramWriter.close(); } catch (_) {}
      try { await writer.close(); } catch (_) {}
      session.close();
    },
  };
}

async function connectWebSocket(config, wasm) {
  const socket = new WebSocket(config.realtimeWebsocketUrl);
  socket.binaryType = "arraybuffer";
  await waitForSocketOpen(socket);
  const authenticationResponse = nextSocketMessage(socket);
  socket.send(wasm.microphone_worker_authenticate_websocket_message(config.accessToken));
  verifyEnvelope(await authenticationResponse, "control", "authenticated");
  const bindResponse = nextSocketMessage(socket);
  socket.send(wasm.microphone_worker_bind_uplink_websocket_message(config.uplinkGrant));
  verifyEnvelope(await bindResponse, "voice_chat", "microphone_uplink_bound");
  return {
    kind: TRANSPORT_WEBSOCKET,
    canSend: () => socket.readyState === WebSocket.OPEN && socket.bufferedAmount < MAX_WEBSOCKET_BUFFERED_BYTES,
    send: async (bytes) => socket.send(bytes),
    close: async () => socket.close(),
  };
}

function createFrameReader(reader) {
  let pending = new Uint8Array(0);
  async function readExact(length) {
    const output = new Uint8Array(length);
    let offset = 0;
    while (offset < length) {
      if (pending.length === 0) {
        const { value, done } = await reader.read();
        if (done || !value) throw new Error("realtime stream closed mid-frame");
        pending = value;
      }
      const count = Math.min(pending.length, length - offset);
      output.set(pending.subarray(0, count), offset);
      pending = pending.subarray(count);
      offset += count;
    }
    return output;
  }
  return {
    async readFrame() {
      const header = await readExact(4);
      const length = new DataView(header.buffer, header.byteOffset, 4).getUint32(0);
      return readExact(length);
    },
  };
}

function verifyEnvelope(value, expectedModule, expectedKind) {
  const envelope = value instanceof Uint8Array
    ? JSON.parse(new TextDecoder().decode(value))
    : JSON.parse(typeof value === "string" ? value : new TextDecoder().decode(value));
  if (envelope.module !== expectedModule || envelope.kind !== expectedKind) {
    throw new Error(`unexpected realtime response ${String(envelope.module)}/${String(envelope.kind)}`);
  }
}

function waitForSocketOpen(socket) {
  return new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", () => reject(new Error("WebSocket connection failed")), { once: true });
  });
}

function nextSocketMessage(socket) {
  return new Promise((resolve, reject) => {
    socket.addEventListener("message", (event) => resolve(event.data), { once: true });
    socket.addEventListener("error", () => reject(new Error("WebSocket control response failed")), { once: true });
  });
}

function validateStartConfig(config) {
  if (!(config.workletPort instanceof MessagePort)) {
    throw new Error("microphone worker start message is missing AudioWorklet port");
  }
  for (const field of [
    "wasmBindgenUrl", "wasmUrl", "realtimeUrl", "realtimeWebsocketUrl",
    "accessToken", "uplinkGrant", "roomId",
  ]) {
    if (typeof config[field] !== "string" || config[field].length === 0) {
      throw new Error(`microphone worker start message is missing ${field}`);
    }
  }
}

function hexToBytes(value) {
  const normalized = value.replaceAll(":", "").replace(/\s+/g, "");
  if (normalized.length % 2 !== 0) throw new Error("invalid realtime certificate hash");
  const bytes = new Uint8Array(normalized.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(normalized.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

function post(kind, payload = {}) {
  self.postMessage({ kind, ...payload });
}

function failActive(message, detail) {
  post("error", { message, detail });
  void stopActive();
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}
