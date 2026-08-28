/**
 * Service worker : l'application est préchargée à l'installation, puis servie
 * depuis le cache. Le HTML passe par le réseau d'abord pour récupérer une
 * nouvelle version dès qu'elle existe, avec repli sur le cache hors ligne.
 */
const VERSION = "__VERSION__";
const CACHE = `nexus-one-${VERSION}`;
const PRECACHE = __PRECACHE__;

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(PRECACHE))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET") return;

  const url = new URL(request.url);
  const sameOrigin = url.origin === self.location.origin;

  // Navigation : réseau d'abord, cache en secours (mode avion, atelier sans réseau).
  if (request.mode === "navigate") {
    event.respondWith(
      fetch(request)
        .then((res) => {
          const copy = res.clone();
          caches.open(CACHE).then((c) => c.put("/", copy));
          return res;
        })
        .catch(() => caches.match("/").then((r) => r ?? caches.match("/index.html"))),
    );
    return;
  }

  // Ressources du site : cache d'abord, elles sont empreintées donc immuables.
  if (sameOrigin) {
    event.respondWith(
      caches.match(request).then(
        (hit) =>
          hit ??
          fetch(request).then((res) => {
            if (res.ok && res.type === "basic") {
              const copy = res.clone();
              caches.open(CACHE).then((c) => c.put(request, copy));
            }
            return res;
          }),
      ),
    );
    return;
  }

  // Polices Google : cache d'abord, réseau en secours, sans bloquer hors ligne.
  if (url.hostname.endsWith("gstatic.com") || url.hostname.endsWith("googleapis.com")) {
    event.respondWith(
      caches.match(request).then(
        (hit) =>
          hit ??
          fetch(request)
            .then((res) => {
              const copy = res.clone();
              caches.open(CACHE).then((c) => c.put(request, copy));
              return res;
            })
            .catch(() => new Response("", { status: 504 })),
      ),
    );
  }
});
