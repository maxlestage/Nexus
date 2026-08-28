/**
 * Compilation du site : Bun regroupe le TSX et le CSS, on injecte les noms de
 * fichiers empreintés dans l'index, puis on génère le service worker avec la
 * liste exacte des ressources à mettre en cache.
 */
import { rm, mkdir, readdir, copyFile, stat } from "node:fs/promises";
import { join, relative } from "node:path";

const OUT = "dist";
const PUBLIC = "public";

await rm(OUT, { recursive: true, force: true });
await mkdir(OUT, { recursive: true });

const result = await Bun.build({
  entrypoints: ["./src/main.tsx"],
  outdir: OUT,
  target: "browser",
  format: "esm",
  minify: true,
  sourcemap: "linked",
  naming: { entry: "[name]-[hash].[ext]", chunk: "[name]-[hash].[ext]", asset: "[name]-[hash].[ext]" },
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
});

if (!result.success) {
  for (const log of result.logs) console.error(log);
  throw new Error("échec de la compilation");
}

const outputs = result.outputs.map((o) => "/" + relative(OUT, o.path).replaceAll("\\", "/"));
const js = outputs.find((p) => p.endsWith(".js"));
const css = outputs.find((p) => p.endsWith(".css"));
if (!js) throw new Error("bundle JavaScript introuvable");

/** Recopie récursivement le dossier public dans dist. */
async function copyDir(from: string, to: string): Promise<string[]> {
  const copied: string[] = [];
  let entries;
  try {
    entries = await readdir(from, { withFileTypes: true });
  } catch {
    return copied;
  }
  await mkdir(to, { recursive: true });
  for (const e of entries) {
    const src = join(from, e.name);
    const dst = join(to, e.name);
    if (e.isDirectory()) {
      copied.push(...(await copyDir(src, dst)));
    } else {
      await copyFile(src, dst);
      copied.push("/" + relative(OUT, dst).replaceAll("\\", "/"));
    }
  }
  return copied;
}

const staticFiles = await copyDir(PUBLIC, OUT);

const html = (await Bun.file("index.html").text())
  .replace("<!--CSS-->", css ? `<link rel="stylesheet" href="${css}" />` : "")
  .replace("<!--JS-->", `<script type="module" src="${js}"></script>`);
await Bun.write(join(OUT, "index.html"), html);

// Le service worker précharge l'app et les schémas, pas les cartes ni les sourcemaps.
const precache = [
  "/",
  ...outputs.filter((p) => !p.endsWith(".map")),
  ...staticFiles.filter((p) => p !== "/sw.js" && !p.endsWith(".map")),
];
const version = Bun.hash(precache.join("|") + html).toString(36);
const sw = (await Bun.file(join(PUBLIC, "sw.js")).text())
  .replace("__VERSION__", version)
  .replace("__PRECACHE__", JSON.stringify(precache, null, 2));
await Bun.write(join(OUT, "sw.js"), sw);

let bytes = 0;
for (const p of [...outputs, ...staticFiles, "/index.html"]) {
  try {
    bytes += (await stat(join(OUT, p))).size;
  } catch {
    /* ignore */
  }
}

console.log(`✓ compilé — ${precache.length} ressources, ${(bytes / 1024).toFixed(0)} Ko`);
console.log(`  js  ${js}`);
if (css) console.log(`  css ${css}`);
console.log(`  sw  version ${version}`);
