const assert = require('node:assert/strict');
const vm = require('node:vm');
const { test } = require('node:test');
const { setImmediate } = require('node:timers/promises');
const { source, clock, flush, stalledFetch } = require('./support.cjs');

function worker(fetch) {
  const time = clock();
  const entries = new Map([
    ['https://cheenhub.ru/', new Response('cached landing')],
    ['https://cheenhub.ru/assets/client.wasm', new Response('cached wasm')],
  ]);
  const cache = {
    async match(request) { return entries.get(typeof request === 'string' ? request : request.url)?.clone(); },
    async put(request, response) {
      const body = await response.arrayBuffer();
      entries.set(typeof request === 'string' ? request : request.url, new Response(body));
    },
  };
  const context = vm.createContext({ URL, Request, Response, ReadableStream, AbortController, setTimeout: time.setTimeout, clearTimeout: time.clearTimeout, console: { info() {}, warn() {} }, self: { location: { href: 'https://cheenhub.ru/sw.js?version=test', origin: 'https://cheenhub.ru' }, addEventListener() {} }, caches: { open: async () => cache, match: cache.match }, fetch });
  vm.runInContext(source('sw.js'), context);
  return { time, context, cache };
}

for (const [handler, url, expected] of [
  ['networkFirstNavigation', 'https://cheenhub.ru/', 'cached landing'],
  ['networkWithRuntimeFallback', 'https://cheenhub.ru/assets/client.wasm', 'cached wasm'],
]) {
  test(`${handler} serves cached content when the network stalls`, async () => {
    const env = worker(stalledFetch);
    let response;
    const pending = env.context[handler](new Request(url)).then(value => { response = value; });

    await env.time.advance(10000);

    assert.ok(response, 'the cached response must be returned within ten seconds');
    assert.equal(await response.text(), expected);
    await pending;
    assert.equal(env.time.pending(), 0);
  });
}

test('successful navigation returns fresh content and clears the deadline', async () => {
  const env = worker(async () => new Response('fresh landing'));

  const response = await env.context.networkFirstNavigation(new Request('https://cheenhub.ru/'));
  await flush();

  assert.equal(await response.text(), 'fresh landing');
  assert.equal(env.time.pending(), 0);
});

test('missing navigation cache returns an error after a stalled request', async () => {
  const env = worker(stalledFetch);
  let response;
  env.context.networkFirstNavigation(new Request('https://cheenhub.ru/app/?uncached')).then(value => { response = value; });

  await env.time.advance(10000);

  assert.ok(response);
  assert.equal(response.status, 503);
});

test('a stalled response body also falls back to the cached wasm', async () => {
  const env = worker(async (_request, { signal }) => new Response(new ReadableStream({
    start(controller) {
      signal.addEventListener('abort', () => controller.error(new Error('aborted body')));
    },
  })));
  let response;
  env.context.networkWithRuntimeFallback(new Request('https://cheenhub.ru/assets/client.wasm')).then(value => { response = value; });
  await flush();
  await setImmediate();

  await env.time.advance(10000);
  await setImmediate();

  assert.ok(response, 'a stalled body must not block the cached bundle');
  assert.equal(await response.text(), 'cached wasm');
});

test('fresh wasm keeps downloading while body chunks arrive for sixteen seconds', async () => {
  let env;
  env = worker(async (_request, { signal }) => new Response(new ReadableStream({
    start(controller) {
      let chunks = 0;
      let timer;
      const send = () => {
        controller.enqueue(new TextEncoder().encode('wasm chunk\n'));
        if (++chunks === 16) controller.close();
        else timer = env.time.setTimeout(send, 1000);
      };
      timer = env.time.setTimeout(send, 1000);
      signal.addEventListener('abort', () => {
        env.time.clearTimeout(timer);
        controller.error(new Error('aborted body'));
      }, { once: true });
    },
  })));
  let response;
  let failure;
  const request = new Request('https://cheenhub.ru/assets/fresh.wasm');
  const pending = env.context.networkWithRuntimeFallback(request).then(
    value => { response = value; },
    error => { failure = error; },
  );

  for (let second = 1; second <= 16; second++) {
    await env.time.advance(1000);
    await setImmediate();
    assert.equal(failure, undefined, `healthy download must survive second ${second}`);
  }
  await pending;

  assert.equal(await response.text(), 'wasm chunk\n'.repeat(16));
  assert.equal(await (await env.cache.match(request)).text(), 'wasm chunk\n'.repeat(16));
  assert.equal(env.time.pending(), 0);
});

test('cache persistence after body completion is not subject to a network timeout', async () => {
  const env = worker(async () => new Response('fresh wasm'));
  const put = env.cache.put;
  env.cache.put = async (request, response) => {
    await put(request, response);
    await new Promise(resolve => env.time.setTimeout(resolve, 15000));
  };
  let response;
  const pending = env.context.networkWithRuntimeFallback(new Request('https://cheenhub.ru/assets/client.wasm')).then(value => { response = value; });
  await flush();
  await setImmediate();

  await env.time.advance(10000);
  assert.equal(response, undefined, 'slow cache persistence must not trigger fallback');
  await env.time.advance(5000);
  await pending;

  assert.equal(await response.text(), 'fresh wasm');
  assert.equal(env.time.pending(), 0);
});

test('body timeout starts again after progress and cancels the stalled download', async () => {
  let env;
  let cancelled = false;
  env = worker(async (_request, { signal }) => new Response(new ReadableStream({
    start(controller) {
      env.time.setTimeout(() => controller.enqueue(new TextEncoder().encode('partial wasm')), 9000);
      signal.addEventListener('abort', () => {
        cancelled = true;
        controller.error(new Error('aborted body'));
      }, { once: true });
    },
    cancel() { cancelled = true; },
  })));
  let response;
  const pending = env.context.networkWithRuntimeFallback(new Request('https://cheenhub.ru/assets/client.wasm')).then(value => { response = value; });

  await env.time.advance(9000);
  await setImmediate();
  await env.time.advance(1000);
  assert.equal(response, undefined, 'recent body progress must reset the timeout');
  await env.time.advance(9000);
  await pending;

  assert.equal(await response.text(), 'cached wasm');
  assert.equal(cancelled, true);
  assert.equal(env.time.pending(), 0);
});

test('request cancellation aborts a pending network fetch without waiting for timeout', async () => {
  const env = worker(stalledFetch);
  const controller = new AbortController();
  const request = new Request('https://cheenhub.ru/assets/client.wasm', { signal: controller.signal });
  let response;
  const pending = env.context.networkWithRuntimeFallback(request).then(value => { response = value; });
  await flush();

  controller.abort();
  await flush();

  assert.ok(response, 'request cancellation must be forwarded immediately');
  assert.equal(await response.text(), 'cached wasm');
  await pending;
  assert.equal(env.time.pending(), 0);
});

test('successful fetch preserves the original response redirect metadata', async () => {
  const fresh = new Response('fresh wasm', { headers: { 'Content-Type': 'application/wasm' } });
  Object.defineProperties(fresh, {
    url: { value: 'https://cheenhub.ru/assets/redirected.wasm' },
    redirected: { value: true },
  });
  const env = worker(async () => fresh);

  const response = await env.context.networkWithRuntimeFallback(new Request('https://cheenhub.ru/assets/client.wasm'));

  assert.equal(response.url, 'https://cheenhub.ru/assets/redirected.wasm');
  assert.equal(response.redirected, true);
  assert.equal(response.headers.get('Content-Type'), 'application/wasm');
  assert.equal(await response.text(), 'fresh wasm');
  assert.equal(env.time.pending(), 0);
});

test('cache storage receives the network response redirect metadata', async () => {
  // Созданный вручную Node Response не имеет URL; fixture воспроизводит
  // сохранение URL и redirected при clone настоящего ответа fetch.
  const withRedirect = response => {
    const clone = response.clone.bind(response);
    Object.defineProperties(response, {
      url: { value: 'https://cheenhub.ru/assets/redirected/module.mjs' },
      redirected: { value: true },
      clone: { value: () => withRedirect(clone()) },
    });
    return response;
  };
  const env = worker(async () => withRedirect(new Response('import "./dep.mjs";')));
  let cached;
  env.cache.put = async (_request, response) => {
    cached = response.clone();
    await response.arrayBuffer();
  };

  await env.context.networkWithRuntimeFallback(new Request('https://cheenhub.ru/assets/module.mjs'));

  assert.equal(cached.url, 'https://cheenhub.ru/assets/redirected/module.mjs');
  assert.equal(cached.redirected, true);
  assert.equal(await cached.text(), 'import "./dep.mjs";');
  assert.equal(env.time.pending(), 0);
});
