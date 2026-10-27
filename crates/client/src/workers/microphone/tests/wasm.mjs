// Запуск: после cargo make build-microphone-worker-wasm, node <этот файл>.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const output = new URL('../../../../public/workers/microphone/', import.meta.url);
const wasm = await import(new URL('microphone_worker.js', output));
await wasm.default({ module_or_path: await readFile(new URL('microphone_worker_bg.wasm', output)) });
assert.equal(wasm.microphone_worker_abi_version(), 9);
const processor = new wasm.MicrophoneWorkerProcessor(48000, 1, 'voice_activated', 0.2, 0, 0, 1, '00000000-0000-4000-8000-000000000001');
let timestamp = 0;
const process = samples => processor.process_pcm(samples, timestamp += 10000);
const tone = new Float32Array(480).fill(0.25);
const firstPacket = processor.voice_datagram(new Uint8Array([1, 2]), 10000, 10000);
const nextPacket = processor.voice_datagram(new Uint8Array([3, 4]), 20000, 10000);
assert.equal(firstPacket[4], 2);
assert.equal(firstPacket.length, 84);
assert.equal(firstPacket[80], 1);
assert.equal(firstPacket[81], 1);
assert.deepEqual(Array.from(firstPacket.slice(82)), [1, 2]);
assert.deepEqual(firstPacket.slice(64, 80), nextPacket.slice(64, 80));
assert.ok(firstPacket.slice(64, 80).some(value => value !== 0));
processor.set_input_gain(0.5);
assert.deepEqual(processor.voice_datagram(new Uint8Array([1]), 30000, 10000).slice(64, 80), firstPacket.slice(64, 80));
processor.set_input_gain(1);
const rms = process(tone);
assert.equal(rms.active, true);
assert.equal(rms.source, "RMS");
assert.equal(rms.threshold, Math.fround(0.2));
processor.set_activation_config('voice_activated', 0.3, 0.5, 0, 80000);
assert.equal(process(tone).active, true, 'изменение порога сохраняет открытый gate');
processor.set_activation_config('always_active', 1, 1, 0, 0);
assert.equal(process(new Float32Array(480)).active, true);
processor.set_activation_config('voice_activated', 1, 1, 0, 0);
assert.equal(process(tone).active, false);
processor.set_activation_config('voice_activated', 1, 0, 0, 0);
processor.set_denoise_enabled(true);
const neural = process(new Float32Array(480));
assert.equal(neural.active, true, 'neural-порог независим от RMS');
assert.equal(neural.source, "RNNoise");
assert.equal(neural.threshold, 0);
processor.set_activation_config('voice_activated', 1, 1, 0, 0);
assert.equal(process(new Float32Array(480)).active, false);
processor.set_activation_config('always_active', 1, 1, 0, 0);
const invalid = new Float32Array(481).fill(0.25);
const failed = process(invalid);
assert.equal(failed.denoiseFailed, true);
assert.equal(failed.source, "RMS");
assert.equal(failed.threshold, 1);
assert.deepEqual(failed.samples, invalid);
processor.set_denoise_enabled(true);
assert.equal(process(tone).denoiseFailed, false, 'повторное включение восстанавливает DSP');
processor.set_denoise_enabled(false);
assert.deepEqual(process(tone).samples, tone);
processor.free();

const messages = [];
const calls = [];
const diagnostics = await import(new URL("../../../../public/audio/microphone-uplink-diagnostics.js", import.meta.url));
const context = vm.createContext({ ...diagnostics, self: { postMessage: message => messages.push(message) }, calls });
const source = await readFile(new URL('../../../../public/audio/microphone-uplink-worker.js', import.meta.url), 'utf8');
vm.runInContext(source.replace(/^import \{ recordPcmInputProfile, emitSendProfile \}[^\n]*\n/, '') + `
active = { closed: false, processor: {
  set_denoise_enabled: value => calls.push(value),
  set_activation_config: (...values) => calls.push(values),
} };
self.onmessage({data:{kind:'set-denoise',denoiseEnabled:true,denoiseRevision:9}});
self.onmessage({data:{kind:'set-activation',activationMode:'voice_activated',vadThreshold:.2,neuralThreshold:.8,vadActivationDelayUs:60000,vadReleaseDelayUs:250000}});
active = null;
self.onmessage({data:{kind:'set-denoise',denoiseEnabled:false,denoiseRevision:13}});
`, context);
assert.equal(messages[0].kind, 'denoise-applied');
assert.equal(messages[0].denoiseRevision, 9);
assert.deepEqual(Array.from(calls[1]), ['voice_activated', .2, .8, 60000, 250000]);
assert.equal(messages.at(-1).kind, 'denoise-rejected');
assert.equal(messages.at(-1).denoiseRevision, 13);
console.log('WASM/worker: live VAD, thresholds, gate preservation, DSP failure/retry and acknowledgements passed');
