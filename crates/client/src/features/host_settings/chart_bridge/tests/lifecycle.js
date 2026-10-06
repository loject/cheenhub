const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

const source = fs.readFileSync(path.join(__dirname, '../../chart_bridge.rs'), 'utf8');
const bridge = source.match(/const BRIDGE: &str = r#"([\s\S]*?)"#;/)[1];

function environment() {
    let element = { canvas: null };
    const window = {
        echarts: {
            init(container) {
                return {
                    getDom: () => container,
                    setOption(option) { container.canvas = option; },
                    resize() {},
                    dispose() { container.canvas = null; }
                };
            }
        }
    };
    vm.runInNewContext(bridge, {
        window,
        document: { getElementById: () => element }
    });
    return {
        charts: window.__cheenhubCharts,
        current: () => element,
        replace() { element = { canvas: null }; },
        remove() { element = null; }
    };
}

test('mount renders into the replacement container after a failed refresh', async () => {
    const env = environment();
    await env.charts.mount('messages', { value: 1 });
    const removed = env.current();

    env.remove();
    env.replace();
    await env.charts.mount('messages', { value: 2 });

    assert.deepEqual(env.current().canvas, { value: 2 });
    assert.equal(removed.canvas, null);
});

test('update rejects an instance whose container has been replaced', async () => {
    const env = environment();
    await env.charts.mount('messages', { value: 1 });
    env.replace();

    assert.equal(env.charts.update('messages', { value: 2 }), false);
    await env.charts.mount('messages', { value: 2 });

    assert.deepEqual(env.current().canvas, { value: 2 });
});
