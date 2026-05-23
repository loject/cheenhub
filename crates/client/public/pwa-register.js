(function () {
  "use strict";

  const LOG_PREFIX = "[pwa]";
  const CACHE_NAME = "cheenhub-pwa-shell-v1";
  const OFFLINE_CHECK_PATH = "/manifest.webmanifest";
  const RETRY_DELAY_MS = 5000;
  let retryTimer = 0;
  let registrationPromise = null;

  const log = {
    debug(message, details) {
      console.debug(LOG_PREFIX, message, details || "");
    },
    info(message, details) {
      console.info(LOG_PREFIX, message, details || "");
    },
    warn(message, details) {
      console.warn(LOG_PREFIX, message, details || "");
    },
  };

  function offlinePanel() {
    return document.getElementById("pwa-offline-fallback");
  }

  function statusNode() {
    return document.getElementById("pwa-offline-status");
  }

  function isOfflineShell() {
    return document.documentElement.dataset.pwaOfflineShell === "true";
  }

  function appHasRendered() {
    const main = document.getElementById("main");
    return Boolean(main && main.childElementCount > 0);
  }

  function shouldShowOfflinePanel() {
    return isOfflineShell() || !appHasRendered();
  }

  function setStatus(message) {
    const node = statusNode();
    if (node) {
      node.textContent = message;
    }
  }

  function showOffline(message) {
    const panel = offlinePanel();
    if (panel) {
      const shouldShow = shouldShowOfflinePanel();
      panel.hidden = !shouldShow;
      panel.setAttribute("aria-hidden", shouldShow ? "false" : "true");
    }
    document.documentElement.dataset.pwaNetwork = "offline";
    setStatus(message || "Ожидаем сеть");
  }

  function hideOffline() {
    const panel = offlinePanel();
    if (panel) {
      panel.hidden = true;
      panel.setAttribute("aria-hidden", "true");
    }
    document.documentElement.dataset.pwaNetwork = "online";
    setStatus("Соединение восстановлено");
  }

  function clearRetryTimer() {
    if (retryTimer) {
      window.clearTimeout(retryTimer);
      retryTimer = 0;
    }
  }

  function scheduleRetry() {
    clearRetryTimer();
    retryTimer = window.setTimeout(() => {
      verifyOnline("timer");
    }, RETRY_DELAY_MS);
  }

  async function canReachOrigin() {
    const url = `${OFFLINE_CHECK_PATH}?cheenhub-online-check=${Date.now()}`;
    const response = await fetch(url, {
      cache: "no-store",
      credentials: "same-origin",
      method: "GET",
    });
    return response.ok;
  }

  async function verifyOnline(source) {
    if (!navigator.onLine) {
      showOffline("Ожидаем сеть");
      scheduleRetry();
      return false;
    }

    setStatus("Проверяем соединение");

    try {
      if (!(await canReachOrigin())) {
        throw new Error("origin check returned a non-success response");
      }

      clearRetryTimer();
      hideOffline();
      log.info("origin is reachable", { source });
      window.dispatchEvent(new CustomEvent("cheenhub:pwa-online"));

      if (isOfflineShell()) {
        window.location.reload();
      }

      return true;
    } catch (error) {
      showOffline("Сеть найдена, CheenHub пока недоступен");
      log.warn("origin reachability check failed", { source, error });
      scheduleRetry();
      return false;
    }
  }

  function normalizeUrl(value, base) {
    try {
      const url = new URL(value, base || window.location.href);
      if (url.origin === window.location.origin) {
        return url.pathname + url.search;
      }
    } catch (_error) {
      return null;
    }

    return null;
  }

  function collectDocumentUrls() {
    const urls = new Set(["/", "/index.html", "/offline.html", "/manifest.webmanifest"]);

    document
      .querySelectorAll("script[src], link[href]")
      .forEach((node) => {
        const source = node.getAttribute("src") || node.getAttribute("href");
        const url = normalizeUrl(source);
        if (url) {
          urls.add(url);
        }
      });

    return urls;
  }

  async function collectModuleDependencies(scriptUrl) {
    const dependencies = new Set();

    try {
      const response = await fetch(scriptUrl, {
        cache: "reload",
        credentials: "same-origin",
      });

      if (!response.ok) {
        return dependencies;
      }

      const source = await response.text();
      const base = new URL(scriptUrl, window.location.href);
      const patterns = [
        /\bfrom\s+["']([^"']+)["']/g,
        /\bimport\s*\(\s*["']([^"']+)["']\s*\)/g,
        /new\s+URL\(\s*["']([^"']+)["']\s*,\s*import\.meta\.url\s*\)/g,
        /module_or_path:\s*["']([^"']+)["']/g,
      ];

      for (const pattern of patterns) {
        let match = pattern.exec(source);
        while (match) {
          const url = normalizeUrl(match[1], base.href);
          if (url) {
            dependencies.add(url);
          }
          match = pattern.exec(source);
        }
      }
    } catch (error) {
      log.warn("failed to inspect module dependencies", { scriptUrl, error });
    }

    return dependencies;
  }

  async function cacheCurrentShell() {
    if (!("caches" in window) || !navigator.onLine) {
      return;
    }

    const urls = collectDocumentUrls();

    for (const script of document.querySelectorAll('script[type="module"][src]')) {
      const scriptUrl = normalizeUrl(script.getAttribute("src"));
      if (!scriptUrl) {
        continue;
      }

      const dependencies = await collectModuleDependencies(scriptUrl);
      dependencies.forEach((dependency) => urls.add(dependency));
    }

    try {
      const cache = await caches.open(CACHE_NAME);
      await Promise.allSettled(
        Array.from(urls).map((url) =>
          cache.add(new Request(url, { credentials: "same-origin" }))
        )
      );
      log.debug("cached current app shell", { urls: Array.from(urls) });

      const registration = await registrationPromise;
      if (registration && registration.active) {
        registration.active.postMessage({
          type: "CACHE_URLS",
          urls: Array.from(urls),
        });
      }
    } catch (error) {
      log.warn("failed to cache current app shell", error);
    }
  }

  async function registerServiceWorker() {
    if (!("serviceWorker" in navigator)) {
      log.warn("service workers are not supported by this browser");
      return null;
    }

    if (!window.isSecureContext) {
      log.warn("service worker registration skipped outside a secure context");
      return null;
    }

    try {
      log.info("registering service worker");
      const registration = await navigator.serviceWorker.register("/sw.js", {
        scope: "/",
        updateViaCache: "none",
      });

      registration.addEventListener("updatefound", () => {
        log.info("service worker update found");
      });

      await navigator.serviceWorker.ready;
      log.info("service worker is ready");
      return registration;
    } catch (error) {
      log.warn("service worker registration failed", error);
      return null;
    }
  }

  function bindOfflineEvents() {
    const main = document.getElementById("main");
    if (main && "MutationObserver" in window) {
      const observer = new MutationObserver(() => {
        if (!navigator.onLine && !isOfflineShell() && appHasRendered()) {
          hideOffline();
          document.documentElement.dataset.pwaNetwork = "offline";
        }
      });
      observer.observe(main, { childList: true });
    }

    window.addEventListener("offline", () => {
      log.info("browser reported offline");
      showOffline("Ожидаем сеть");
      scheduleRetry();
    });

    window.addEventListener("online", () => {
      log.info("browser reported online");
      verifyOnline("online-event");
    });

    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "visible" && !navigator.onLine) {
        showOffline("Ожидаем сеть");
      } else if (document.visibilityState === "visible") {
        verifyOnline("visibilitychange");
      }
    });

    document.addEventListener("click", (event) => {
      if (event.target && event.target.id === "pwa-offline-retry") {
        verifyOnline("retry-button");
      }
    });
  }

  bindOfflineEvents();

  if (!navigator.onLine || isOfflineShell()) {
    showOffline("Ожидаем сеть");
  }

  registrationPromise = registerServiceWorker();

  window.addEventListener("load", () => {
    verifyOnline("initial-load");
    registrationPromise
      .then(() => cacheCurrentShell())
      .catch((error) => log.warn("pwa startup failed", error));
  });
})();
