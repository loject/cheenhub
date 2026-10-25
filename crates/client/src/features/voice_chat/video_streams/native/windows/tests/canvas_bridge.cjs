// Запуск: node --test crates/client/src/features/voice_chat/video_streams/native/windows/tests/canvas_bridge.cjs
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "../../windows.rs"), "utf8");
const script = source.match(/const CANVAS_BRIDGE_SCRIPT: &str = r#"([\s\S]*?)"#;/)[1];

async function startBridge({ drawError, sendError } = {}) {
    const events = [];
    const uncaught = [];
    const draws = [];
    const closedFrames = [];
    const pending = ["video-target"];
    let waiting;
    let callbacks;
    let removed = false;
    const canvas = {
        setAttribute() {},
        getContext() {
            return { drawImage(...args) {
                if (drawError) throw drawError;
                draws.push(args);
            } };
        },
        remove() { removed = true; },
    };
    const context = vm.createContext({
        dioxus: {
            recv() {
                if (pending.length) return Promise.resolve(pending.shift());
                return new Promise(resolve => { waiting = resolve; });
            },
            // NativeDioxusChannel.send синхронно отправляет IPC и возвращает undefined.
            send(event) {
                if (sendError) throw sendError;
                events.push(event);
            },
        },
        document: {
            getElementById() { return { appendChild() {} }; },
            createElement() { return canvas; },
        },
        EncodedVideoChunk: class {},
        VideoDecoder: class {
            static async isConfigSupported() { return { supported: true }; }
            constructor(init) { callbacks = init; this.state = "unconfigured"; }
            configure() { this.state = "configured"; }
            close() { this.state = "closed"; }
        },
    });
    const completion = vm.runInContext("(async () => {" + script + "})()", context);
    await new Promise(resolve => setImmediate(resolve));
    assert.ok(callbacks, "bridge creates a decoder");

    return {
        events, uncaught, draws, closedFrames,
        output(width = 1920, height = 1080) {
            const frame = { displayWidth: width, displayHeight: height, close() { closedFrames.push(frame); } };
            try { callbacks.output(frame); } catch (error) { uncaught.push(String(error)); }
            return frame;
        },
        decoderError(error) {
            try { callbacks.error(error); } catch (error) { uncaught.push(String(error)); }
        },
        async close() {
            waiting({ kind: "close" });
            assert.equal(await completion, true);
            assert.equal(removed, true);
        },
    };
}

test("first_frame_is_drawn_after_synchronous_dimension_notification", async () => {
    const bridge = await startBridge();

    const frame = bridge.output();

    assert.equal(bridge.draws.length, 1);
    assert.deepEqual(bridge.draws[0], [frame, 0, 0, 1920, 1080]);
    assert.deepEqual(JSON.parse(JSON.stringify(bridge.events)), [{ width: 1920, height: 1080 }]);
    assert.deepEqual(bridge.closedFrames, [frame]);
    assert.deepEqual(bridge.uncaught, []);
    await bridge.close();
});

test("drawing_error_is_reported_without_throwing_and_frame_is_closed", async () => {
    const bridge = await startBridge({ drawError: new Error("draw failed") });

    const frame = bridge.output();

    assert.equal(bridge.events[1], "Error: draw failed");
    assert.deepEqual(bridge.closedFrames, [frame]);
    assert.deepEqual(bridge.uncaught, []);
    await bridge.close();
});

test("decoder_error_is_reported_without_a_secondary_exception", async () => {
    const bridge = await startBridge();

    bridge.decoderError(new Error("decoder failed"));

    assert.deepEqual(bridge.events, ["Error: decoder failed"]);
    assert.deepEqual(bridge.uncaught, []);
    await bridge.close();
});

test("unavailable_event_receiver_does_not_prevent_drawing_or_frame_cleanup", async () => {
    const bridge = await startBridge({ sendError: new Error("receiver closed") });

    const frame = bridge.output();

    assert.equal(bridge.draws.length, 1);
    assert.deepEqual(bridge.closedFrames, [frame]);
    assert.deepEqual(bridge.uncaught, []);
    await bridge.close();
});
