const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const publicDirectory = path.resolve(__dirname, '../../../../../public');

function clock() {
  let now = 0;
  let id = 0;
  const timers = new Map();
  return {
    now: () => now,
    setTimeout(callback, delay) { timers.set(++id, { callback, at: now + delay }); return id; },
    clearTimeout(id) { timers.delete(id); },
    async advance(duration) {
      const end = now + duration;
      await flush();
      while (true) {
        const next = [...timers].filter(([, timer]) => timer.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
        if (!next) break;
        now = next[1].at;
        timers.delete(next[0]);
        next[1].callback();
        await flush();
      }
      now = end;
      await flush();
    },
    pending: () => timers.size,
  };
}

async function flush() {
  for (let i = 0; i < 30; i++) await Promise.resolve();
}

function source(file) { return fs.readFileSync(path.join(publicDirectory, file), 'utf8'); }
function stalledFetch(_request, options) {
  return new Promise((resolve, reject) => {
    options?.signal?.addEventListener('abort', () => reject(new Error('aborted')));
  });
}

function browser({ rendered = false, standalone = false, pathname = '/', storage = new Map(), fetch = async () => new Response('ok'), offlineShell = false, hidden = false } = {}) {
  const time = clock();
  const windowListeners = new Map();
  const documentListeners = new Map();
  const observers = [];
  const main = { childElementCount: Number(rendered) };
  let reloads = 0;
  let requests = 0;
  const panel = { hidden: true, setAttribute() {} };
  const add = (listeners, kind, handler) => {
    if (!listeners.has(kind)) listeners.set(kind, []);
    listeners.get(kind).push(handler);
  };
  const navigator = { onLine: true };
  const document = {
    visibilityState: hidden ? 'hidden' : 'visible',
    documentElement: { dataset: { pwaOfflineShell: String(offlineShell) } },
    getElementById(id) { return id === 'main' ? main : id === 'pwa-offline-fallback' ? panel : { textContent: '' }; },
    querySelectorAll() { return []; },
    addEventListener(kind, handler) { add(documentListeners, kind, handler); },
  };
  class MutationObserver {
    constructor(callback) { observers.push(callback); }
    observe() {}
  }
  const window = {
    navigator,
    location: { pathname, href: `https://cheenhub.ru${pathname}`, origin: 'https://cheenhub.ru', reload() { reloads++; } },
    sessionStorage: { getItem(key) { return storage.get(key) ?? null; }, setItem(key, value) { storage.set(key, value); }, removeItem(key) { storage.delete(key); } },
    matchMedia() { return { matches: standalone }; },
    setTimeout: time.setTimeout,
    clearTimeout: time.clearTimeout,
    MutationObserver,
    addEventListener(kind, handler) { add(windowListeners, kind, handler); },
    dispatchEvent(event) { for (const handler of windowListeners.get(event.type) || []) handler(event); },
  };
  const context = vm.createContext({ window, document, navigator, MutationObserver, URL, AbortController, Date: class extends Date { static now() { return time.now(); } }, CustomEvent: class { constructor(type, options) { this.type = type; this.detail = options?.detail; } }, console: { debug() {}, info() {}, warn() {} }, fetch(...args) { requests++; return fetch(...args); } });
  vm.runInContext(source('pwa-register.js'), context);
  return {
    time, window, document, navigator, storage, panel,
    reloads: () => reloads,
    requests: () => requests,
    async event(type) {
      window.dispatchEvent({ type });
      for (const handler of documentListeners.get(type) || []) handler({ type });
      await flush();
    },
    async render() { main.childElementCount = 1; for (const callback of observers) callback(); await flush(); },
  };
}

module.exports = { source, clock, flush, browser, stalledFetch };
