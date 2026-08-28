/**
 * Serveur de production : Bun sert les fichiers compilés et renvoie l'index
 * pour toute route inconnue (application à page unique).
 *
 * Heroku impose le port par la variable d'environnement PORT.
 */
import { join, normalize } from "node:path";

const DIST = "dist";
const port = Number(process.env.PORT ?? 3000);

const IMMUTABLE = /-[A-Za-z0-9]{6,}\.(js|css|woff2?)$/;

const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".webmanifest": "application/manifest+json",
  ".json": "application/json",
  ".map": "application/json",
  ".ico": "image/x-icon",
  ".txt": "text/plain; charset=utf-8",
};

function contentType(path: string): string | undefined {
  const dot = path.lastIndexOf(".");
  return dot === -1 ? undefined : TYPES[path.slice(dot)];
}

function securityHeaders(headers: Headers): Headers {
  headers.set("X-Content-Type-Options", "nosniff");
  headers.set("Referrer-Policy", "strict-origin-when-cross-origin");
  headers.set("X-Frame-Options", "SAMEORIGIN");
  return headers;
}

const server = Bun.serve({
  port,
  hostname: "0.0.0.0",
  idleTimeout: 30,

  async fetch(req) {
    const url = new URL(req.url);

    if (req.method !== "GET" && req.method !== "HEAD") {
      return new Response("Méthode non autorisée", { status: 405, headers: { Allow: "GET, HEAD" } });
    }

    if (url.pathname === "/healthz") {
      return new Response("ok", { headers: { "Content-Type": "text/plain" } });
    }

    // normalize neutralise les tentatives de remontée de dossier (../).
    const rel = normalize(decodeURIComponent(url.pathname)).replace(/^(\.\.[/\\])+/, "");
    let file = Bun.file(join(DIST, rel));

    if (rel === "/" || rel === "\\" || !(await file.exists())) {
      // Une ressource manquante avec extension est une vraie 404 ;
      // le reste retombe sur l'application.
      if (rel !== "/" && contentType(rel) && rel !== "/index.html") {
        return new Response("Introuvable", { status: 404 });
      }
      file = Bun.file(join(DIST, "index.html"));
      if (!(await file.exists())) {
        return new Response("Site non compilé — lancez « bun run build ».", { status: 500 });
      }
    }

    const headers = securityHeaders(new Headers());
    const type = contentType(file.name ?? rel);
    if (type) headers.set("Content-Type", type);

    if (IMMUTABLE.test(rel)) {
      headers.set("Cache-Control", "public, max-age=31536000, immutable");
    } else if (rel.endsWith("/sw.js") || rel === "/sw.js") {
      headers.set("Cache-Control", "no-cache");
    } else if (type?.startsWith("text/html")) {
      headers.set("Cache-Control", "no-cache");
    } else {
      headers.set("Cache-Control", "public, max-age=3600");
    }

    return new Response(file, { headers });
  },
});

console.log(`Nexus One — écoute sur http://${server.hostname}:${server.port}`);
