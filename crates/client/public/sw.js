const CACHE_VERSION = "v1";
const SHELL_CACHE = `cheenhub-pwa-shell-${CACHE_VERSION}`;
const RUNTIME_CACHE = `cheenhub-pwa-runtime-${CACHE_VERSION}`;
const CACHE_NAMES = new Set([SHELL_CACHE, RUNTIME_CACHE]);
const CORE_ASSETS = [
  "/",
  "/index.html",
  "/offline.html",
  "/manifest.webmanifest",
  "/pwa-register.js",
  "/icons/favicon.svg",
  "/icons/icon-144.png",
  "/icons/icon-192.png",
  "/icons/icon-512.png",
  "/icons/maskable-512.png",
  "/icons/apple-touch-icon.png",
  "/screenshots/install-wide.png",
  "/screenshots/install-mobile.png",
];

function logInfo(message, details) {
  console.info("[pwa]", message, details || "");
}

function logWarn(message, details) {
  console.warn("[pwa]", message, details || "");
}

function sameOrigin(url) {
  return url.origin === self.location.origin;
}

async function cacheUrls(cacheName, urls) {
  const cache = await caches.open(cacheName);
  const uniqueUrls = Array.from(new Set(urls));

  await Promise.allSettled(
    uniqueUrls.map((url) =>
      cache.add(new Request(url, { credentials: "same-origin" }))
    )
  );
}

self.addEventListener("install", (event) => {
  logInfo("installing service worker", { cache: SHELL_CACHE });
  self.skipWaiting();
  event.waitUntil(cacheUrls(SHELL_CACHE, CORE_ASSETS));
});

self.addEventListener("activate", (event) => {
  logInfo("activating service worker", { cache: SHELL_CACHE });
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => key.startsWith("cheenhub-pwa-") && !CACHE_NAMES.has(key))
            .map((key) => caches.delete(key))
        )
      )
      .then(() => self.clients.claim())
  );
});

self.addEventListener("message", (event) => {
  const message = event.data || {};

  if (message.type === "SKIP_WAITING") {
    self.skipWaiting();
    return;
  }

  if (message.type === "CACHE_URLS" && Array.isArray(message.urls)) {
    event.waitUntil(cacheUrls(SHELL_CACHE, message.urls));
  }
});

function shouldBypass(request, url) {
  return (
    request.method !== "GET" ||
    !sameOrigin(url) ||
    url.pathname.startsWith("/api/") ||
    url.pathname.startsWith("/realtime") ||
    url.searchParams.has("cheenhub-online-check") ||
    request.cache === "no-store"
  );
}

function isStaticAsset(url) {
  return (
    url.pathname.startsWith("/assets/") ||
    url.pathname.startsWith("/wasm/") ||
    url.pathname.startsWith("/icons/") ||
    url.pathname.startsWith("/screenshots/") ||
    url.pathname === "/manifest.webmanifest" ||
    url.pathname === "/pwa-register.js"
  );
}

async function networkFirstNavigation(request) {
  const cache = await caches.open(SHELL_CACHE);

  try {
    const response = await fetch(request);
    if (response.ok) {
      await cache.put(request, response.clone());
      await cache.put("/", response.clone());
    }
    return response;
  } catch (error) {
    const cached =
      (await cache.match(request)) ||
      (await cache.match("/")) ||
      (await cache.match("/index.html")) ||
      (await cache.match("/offline.html"));

    if (cached) {
      return cached;
    }

    logWarn("navigation fallback cache miss", error);
    return new Response("CheenHub is offline and the app shell is not cached.", {
      status: 503,
      headers: { "Content-Type": "text/plain; charset=utf-8" },
    });
  }
}

async function cacheFirstAsset(request) {
  const cached = await caches.match(request);
  if (cached) {
    return cached;
  }

  const response = await fetch(request);
  if (response.ok) {
    const cache = await caches.open(RUNTIME_CACHE);
    await cache.put(request, response.clone());
  }
  return response;
}

async function networkWithRuntimeFallback(request) {
  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(RUNTIME_CACHE);
      await cache.put(request, response.clone());
    }
    return response;
  } catch (_error) {
    const cached = await caches.match(request);
    if (cached) {
      return cached;
    }
    throw _error;
  }
}

self.addEventListener("fetch", (event) => {
  const { request } = event;
  const url = new URL(request.url);

  if (shouldBypass(request, url)) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(networkFirstNavigation(request));
    return;
  }

  if (isStaticAsset(url)) {
    event.respondWith(cacheFirstAsset(request));
    return;
  }

  event.respondWith(networkWithRuntimeFallback(request));
});
