import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import vm from "node:vm";

const source = await readFile(new URL("../../../../../../public/audio/screen-audio-worklet.js", import.meta.url), "utf8");
function capture() {
  const messages = [];
  let Processor;
  const context = vm.createContext({
    sampleRate: 48000, currentFrame: 0,
    AudioWorkletProcessor: class {
      constructor() {
        this.port = {postMessage: (data, transfer) => messages.push(structuredClone(data, {transfer}))};
      }
    },
    registerProcessor: (name, type) => {
      assert.equal(name, "cheenhub-screen-audio");
      Processor = type;
    },
  });
  vm.runInContext(source, context);
  const processor = new Processor();
  function render(channels, count) {
    for (let i = 0; i < count; i++) {
      const output = new Float32Array(128).fill(1);
      assert.equal(processor.process([channels], [[output]]), true);
      assert.ok(output.every(sample => sample === 0), "захват не должен воспроизводить системный звук повторно");
      context.currentFrame += 128;
    }
  }
  return {processor, messages, render};
}

const left = new Float32Array(128).fill(0.25);
const right = new Float32Array(128).fill(-0.5);
const stereo = capture();
stereo.render([left, right], 8);
assert.equal(stereo.messages.length, 1);
assert.equal(stereo.messages[0].samples.length, 1920);
assert.ok(stereo.messages[0].samples.slice(0, 960).every(sample => sample === 0.25));
assert.ok(stereo.messages[0].samples.slice(960).every(sample => sample === -0.5));
assert.equal(stereo.messages[0].timestampUs, 0);

const mono = capture();
mono.render([left], 8);
assert.ok(mono.messages[0].samples.every(sample => sample === 0.25));

const slow = capture();
slow.render([left, right], 32);
assert.equal(slow.messages.length, 2, "медленный consumer не должен накапливать PCM");
slow.processor.port.onmessage({data: "consumed"});
slow.render([left, right], 8);
assert.equal(slow.messages.length, 3, "подтверждение возобновляет передачу PCM");
assert.ok(slow.messages[2].timestampUs > 40000, "пропуск PCM сохраняет реальное время захвата");
slow.processor.port.onmessage({data: {kind: "stop"}});
assert.equal(slow.processor.process([[left, right]], []), false);
console.log("Screen audio worklet: stereo, mono, silent output, bounded queue, resume, timestamps and stop passed");
