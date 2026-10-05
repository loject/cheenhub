const assert = require('node:assert/strict');
const { test } = require('node:test');
const { browser, stalledFetch } = require('./support.cjs');

test('empty landing recovers even when the window load event never fires', async () => {
  const env = browser();

  await env.time.advance(20000);

  assert.equal(env.reloads(), 1);
});

test('a rendered ordinary tab verifies connectivity on return without reloading', async () => {
  const env = browser({ rendered: true, pathname: '/app/' });

  await env.event('visibilitychange');

  assert.equal(env.requests(), 1);
  assert.equal(env.reloads(), 0);
});

test('a recovered page cannot enter an automatic reload loop', async () => {
  const storage = new Map();
  const first = browser({ storage });
  await first.time.advance(20000);
  assert.equal(first.reloads(), 1);

  const second = browser({ storage });
  await second.time.advance(60000);
  await second.event('online');

  assert.equal(second.reloads(), 0);
});

test('rendering clears the recovery limit for a future failed startup', async () => {
  const storage = new Map();
  const first = browser({ storage });
  await first.time.advance(20000);
  assert.equal(first.reloads(), 1);
  const healthy = browser({ storage });

  await healthy.render();
  await healthy.time.advance(20000);
  const future = browser({ storage });
  await future.time.advance(20000);

  assert.equal(healthy.reloads(), 0);
  assert.equal(future.reloads(), 1);
});

test('an offline landing waits for the network before recovering', async () => {
  const env = browser();
  env.navigator.onLine = false;

  await env.time.advance(30000);
  assert.equal(env.reloads(), 0);
  env.navigator.onLine = true;
  await env.event('online');

  assert.equal(env.reloads(), 1);
});

test('a hidden empty tab recovers only after it becomes visible', async () => {
  const env = browser({ hidden: true });

  await env.time.advance(30000);
  assert.equal(env.reloads(), 0);
  env.document.visibilityState = 'visible';
  await env.event('visibilitychange');

  assert.equal(env.reloads(), 1);
});

test('a stalled connectivity probe times out and retries', async () => {
  let reachable = false;
  const env = browser({ fetch: (...args) => reachable ? Promise.resolve(new Response('ok')) : stalledFetch(...args) });

  await env.event('online');
  await env.time.advance(10000);
  assert.equal(env.reloads(), 0);
  reachable = true;
  await env.time.advance(20000);

  assert.equal(env.reloads(), 1);
});

test('offline app shell still reloads as soon as connectivity returns', async () => {
  const env = browser({ offlineShell: true });

  await env.event('online');

  assert.equal(env.reloads(), 1);
});

test('slow but successful startup does not reload before the grace period', async () => {
  const env = browser();

  await env.time.advance(15000);
  await env.render();
  await env.time.advance(30000);

  assert.equal(env.reloads(), 0);
});

test('concurrent resume events share one connectivity probe', async () => {
  let finishProbe;
  const env = browser({ rendered: true, fetch: () => new Promise(resolve => { finishProbe = resolve; }) });

  await env.event('online');
  await env.event('visibilitychange');
  assert.equal(env.requests(), 1);
  finishProbe(new Response('ok'));
  await env.time.advance(5000);

  assert.equal(env.reloads(), 0);
  assert.equal(env.time.pending(), 0);
});

test('blocked session storage prevents unbounded automatic recovery', async () => {
  const env = browser();
  env.window.sessionStorage.setItem = () => { throw new Error('storage denied'); };

  await env.time.advance(60000);
  await env.event('online');

  assert.equal(env.reloads(), 0);
});

test('an already rendered installed application is preserved on resume', async () => {
  const env = browser({ rendered: true, standalone: true, pathname: '/app/' });

  await env.time.advance(30000);
  await env.event('visibilitychange');

  assert.equal(env.requests(), 1);
  assert.equal(env.reloads(), 0);
});
