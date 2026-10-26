import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const workletSource = readFileSync(new URL('../microphone-worklet.js', import.meta.url), 'utf8');
const workerSource = readFileSync(new URL('../microphone-uplink-worker.js', import.meta.url), 'utf8')
  .replace(/^import \{ recordPcmInputProfile, emitSendProfile \}[^\n]*\n/, '');

function createWorklet(frameCount = 480) {
  const messages = [];
  const port = { onmessage: null, postMessage: message => messages.push(message) };
  let Processor;
  class FakeAudioWorkletProcessor {
    constructor() { this.port = port; }
  }
  vm.runInNewContext(workletSource, {
    AudioWorkletProcessor: FakeAudioWorkletProcessor,
    Float32Array,
    registerProcessor: (_, value) => { Processor = value; },
    sampleRate: 48_000,
  });
  return {
    processor: new Processor({ processorOptions: { frameCount, diagnosticsEnabled: false } }),
    port,
    messages,
  };
}

function sendFrame(processor, frameCount = 480) {
  processor.process([[new Float32Array(frameCount).fill(0.2)]], []);
}

function createWorker() {
  const notifications = [];
  const acknowledged = [];
  const encoded = [];
  const processed = [];
  class FakeAudioData {
    constructor(options) { this.timestamp = options.timestamp; }
    close() {}
  }
  const context = {
    AudioData: FakeAudioData,
    Date: { now: () => 2_000_000 },
    Float32Array,
    performance: { now: () => 2_000_000 },
    recordPcmInputProfile: () => {},
    emitSendProfile: () => {},
    self: { postMessage: message => notifications.push(message) },
  };
  vm.createContext(context);
  vm.runInContext(workerSource, context);
  context.current = {
    closed: false,
    port: { postMessage: message => acknowledged.push(message) },
    encoder: { encodeQueueSize: 0, encode: audio => encoded.push(audio.timestamp) },
    processor: {
      process_pcm: (_, timestampUs) => {
        processed.push(timestampUs);
        return {
          active: true,
          samples: new Float32Array(480).fill(0.2),
          timestampUs,
          rms: 0.2,
          threshold: 0.09,
          source: 'RNNoise',
        };
      },
    },
    lastLevelActive: false,
    lastLevelMs: 0,
    droppedPcm: 0,
    droppedEncoded: 0,
    lastWarningMs: 0,
    diagnosticsEnabled: false,
  };
  return { context, notifications, acknowledged, encoded, processed };
}

test('worklet_keeps_500ms_of_pcm_and_resumes_with_latest_frame', () => {
  const { processor, port, messages } = createWorklet();
  port.onmessage({ data: { kind: 'enable-backpressure' } });

  for (let frame = 0; frame < 60; frame += 1) sendFrame(processor);

  const firstFrames = messages.filter(message => message.samples instanceof Float32Array);
  assert.equal(firstFrames.length, 50);

  port.onmessage({ data: { kind: 'pcm-consumed' } });
  sendFrame(processor);

  const frames = messages.filter(message => message.samples instanceof Float32Array);
  assert.equal(frames.length, 51);
  assert.equal(frames[50].timestampUs, 600_000);
});

test('worklet_limits_20ms_frames_to_500ms_of_pcm', () => {
  const { processor, port, messages } = createWorklet(960);
  port.onmessage({ data: { kind: 'enable-backpressure' } });

  for (let frame = 0; frame < 30; frame += 1) sendFrame(processor, 960);

  const firstFrames = messages.filter(message => message.samples instanceof Float32Array);
  assert.equal(firstFrames.length, 25);

  port.onmessage({ data: { kind: 'pcm-consumed' } });
  sendFrame(processor, 960);

  const frames = messages.filter(message => message.samples instanceof Float32Array);
  assert.equal(frames.length, 26);
  assert.equal(frames[25].timestampUs, 600_000);
});

test('worklet_without_dedicated_worker_keeps_main_thread_capture_running', () => {
  const { processor, messages } = createWorklet();

  for (let frame = 0; frame < 10; frame += 1) sendFrame(processor);

  const frames = messages.filter(message => message.samples instanceof Float32Array);
  assert.equal(frames.length, 10, 'предпросмотр не требует ACK от dedicated worker');
});

test('worker_accepts_fresh_pcm_despite_unrelated_wall_clock_offset', () => {
  const { context, acknowledged, encoded, processed } = createWorker();

  vm.runInContext('handleWorkletMessage({ samples: new Float32Array(480), timestampUs: 0 }, current)', context);

  assert.deepEqual(processed, [0]);
  assert.deepEqual(encoded, [0]);
  assert.equal(acknowledged.length, 1);
  assert.equal(acknowledged[0].kind, 'pcm-consumed');
});

test('worker_releases_slot_when_processing_throws', () => {
  const { context, acknowledged, notifications } = createWorker();
  context.current.processor.process_pcm = () => { throw new Error('processor failed'); };

  vm.runInContext('handleWorkletMessage({ samples: new Float32Array(480), timestampUs: 0 }, current)', context);

  assert.equal(acknowledged.length, 1);
  assert.equal(acknowledged[0].kind, 'pcm-consumed');
  assert.equal(notifications.at(-1).kind, 'warning');
});
