// Builds the static site into `dist/`.
//
// Two inputs, on purpose:
//   - `src/`     hand-written HTML, CSS and JS. Nothing generates it.
//   - `content/` Markdown pages, and `../dist/catalog.json` for the devices.
//
// The catalog comes from the device files through `cargo run -p xtask --
// catalog`. The site never restates a device fact that the YAML carries.

import { cp, mkdir, readFile, readdir, rename, rm, writeFile } from "node:fs/promises";
import { createReadStream, existsSync, statSync, watch } from "node:fs";
import { createServer } from "node:http";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Marked } from "marked";

const marked = new Marked({ async: false });

const root = dirname(fileURLToPath(import.meta.url));
const repo = resolve(root, "..");
const dist = join(root, "dist");
// The build writes here and the result moves into `dist/` in one step. A
// reload during a rebuild then reaches the old site or the new one, never a
// directory that is half written. The name carries the process id, so a
// manual build beside a running `npm run dev` does not delete its staging
// directory under it.
const out = join(root, `.dist-build-${process.pid}`);

// The repository page lives under a sub-path. `BASE=/ npm run dev` serves it
// from a root instead.
const PAGES_BASE = "/govee-toolkit/";
const base = (process.env.BASE ?? PAGES_BASE).replace(/\/*$/, "/");
const repoUrl = "https://github.com/damient/govee-toolkit";
const catalogPath = join(repo, "dist/catalog.json");

const pages = [
  { src: "index.html", url: "", nav: "home", title: null, klass: "is-home" },
  { src: "devices.html", url: "devices/", nav: "devices", title: "Devices" },
];

// The reference page sits inside the documentation menu, between the two
// Markdown pages that surround it.
const REFERENCE = { url: "reference/", title: "Reference", order: 3 };

const LANGUAGES = [
  { id: "cli", label: "Command line", state: "ok" },
  { id: "rust", label: "Rust", state: "ok" },
  { id: "python", label: "Python", state: "soon" },
  { id: "node", label: "Node.js", state: "soon" },
];

async function main() {
  const catalog = await readCatalog();
  await rm(out, { recursive: true, force: true });
  await mkdir(out, { recursive: true });

  await cp(join(root, "src/assets"), join(out, "assets"), { recursive: true });
  if (existsSync(join(root, "public"))) {
    await cp(join(root, "public"), out, { recursive: true });
  }
  await writeFile(join(out, ".nojekyll"), "");

  const layout = await readFile(join(root, "src/layout.html"), "utf8");
  const docs = await readDocs();
  const reference = JSON.parse(await readFile(join(root, "content/reference.json"), "utf8"));
  const nav = [
    ...docs.map((d) => ({ url: `docs/${d.slug}/`, title: d.title, order: d.order })),
    REFERENCE,
  ].sort((a, b) => a.order - b.order);

  for (const page of pages) {
    let body = await readFile(join(root, "src/pages", page.src), "utf8");
    if (page.src === "devices.html") body = renderDevices(body, catalog);
    await emit(layout, page, body, { docs: nav, catalog });
  }
  for (const doc of docs) {
    const page = {
      url: `docs/${doc.slug}/`,
      nav: "docs",
      title: doc.title,
      description: doc.description,
      klass: "is-doc",
    };
    const body = docShell(doc, nav);
    await emit(layout, page, body, { docs: nav, catalog });
  }
  await emit(layout, {
    url: REFERENCE.url,
    nav: "docs",
    title: REFERENCE.title,
    description: reference.intro,
    klass: "is-doc",
  }, referenceShell(reference, nav), { docs: nav, catalog });

  await emit(layout, { url: "404.html", nav: "", title: "Page not found" },
    `<section class="slab"><h1>Page not found</h1><p class="lede">That page does not exist. <a href="${base}">Go back to the start</a>.</p></section>`,
    { docs: nav, catalog });

  await publish();
  const clock = new Date().toTimeString().slice(0, 8);
  console.log(`${clock}  site -> ${relative(repo, dist)} (${pages.length + docs.length + 2} pages, ${catalog.devices.length} devices)`);
}

// Two renames, so that `dist/` is missing for microseconds instead of for the
// length of a build.
async function publish() {
  const previous = `${dist}.previous`;
  await rm(previous, { recursive: true, force: true });
  if (existsSync(dist)) await rename(dist, previous);
  await rename(out, dist);
  await rm(previous, { recursive: true, force: true });
}

async function emit(layout, page, body, ctx) {
  const html = fill(layout, {
    base,
    repo: repoUrl,
    lang: "en",
    title: page.title ? `${page.title} — govee-toolkit` : "govee-toolkit — control your Govee lights locally",
    description: page.description ?? "An unofficial toolkit that controls Govee lights over your own network, from Rust, Python, Node.js or the command line.",
    bodyclass: page.klass ?? "",
    canonical: `${base}${page.url}`,
    nav: topNav(page.nav),
    content: fill(body, { base, repo: repoUrl, ...counts(ctx.catalog) }),
    year: String(new Date().getUTCFullYear()),
  });
  const file = page.url.endsWith(".html")
    ? join(out, page.url)
    : join(out, page.url, "index.html");
  await mkdir(dirname(file), { recursive: true });
  await writeFile(file, html);
}

function fill(template, vars) {
  return template.replace(/\{\{\s*([\w.]+)\s*\}\}/g, (all, key) =>
    key in vars ? String(vars[key]) : all);
}

function counts(catalog) {
  const verified = catalog.devices.filter((d) => d.verified?.date).length;
  return { verified_count: String(verified), device_count: String(catalog.devices.length) };
}

function topNav(current) {
  const items = [
    ["", "Home"],
    ["docs/start/", "Docs"],
    ["devices/", "Devices"],
  ];
  // The reference page sits inside the documentation, so the top bar marks
  // Docs while the reader is on it.
  const keys = { "": "home", "docs/start/": "docs", "devices/": "devices" };
  return items
    .map(([url, label]) => {
      const on = keys[url] === current ? ' aria-current="page"' : "";
      return `<a href="${base}${url}"${on}>${label}</a>`;
    })
    .join("\n        ");
}

// --- Markdown -------------------------------------------------------------

async function readDocs() {
  const dir = join(root, "content/docs");
  const files = (await readdir(dir)).filter((f) => f.endsWith(".md"));
  const docs = [];
  for (const file of files) {
    const raw = await readFile(join(dir, file), "utf8");
    const { meta, body } = frontMatter(raw);
    const headings = [];
    const md = new Marked({ async: false });
    md.use({
      walkTokens(token) {
        if (token.type === "heading" && token.depth === 2) {
          const label = headingLabel(token.tokens);
          headings.push({ id: slugify(label), text: label });
        }
      },
      renderer: {
        heading({ text, depth, tokens }) {
          const inner = this.parser.parseInline(tokens);
          if (depth !== 2) return `<h${depth}>${inner}</h${depth}>\n`;
          const id = slugify(headingLabel(tokens));
          return `<h2 id="${id}"><a class="anchor" href="#${id}">${inner}</a></h2>\n`;
        },
      },
    });
    docs.push({
      slug: meta.slug ?? file.replace(/\.md$/, ""),
      title: meta.title ?? file,
      description: meta.description ?? "",
      order: Number(meta.order ?? 99),
      headings,
      html: md.parse(fill(body, { base, repo: repoUrl })),
    });
  }
  return docs.sort((a, b) => a.order - b.order);
}

function slugify(text) {
  return text.toLowerCase().replace(/[^\w]+/g, "-").replace(/^-|-$/g, "");
}

// A heading can end with inline HTML, a state badge for one. The anchor and
// the table of contents take what stands before it.
function headingLabel(tokens = []) {
  const words = [];
  for (const token of tokens) {
    if (token.type === "html") break;
    words.push(token.text ?? token.raw ?? "");
  }
  return words.join("").trim();
}

function frontMatter(raw) {
  const match = raw.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!match) return { meta: {}, body: raw };
  const meta = {};
  for (const line of match[1].split("\n")) {
    const at = line.indexOf(":");
    if (at > 0) meta[line.slice(0, at).trim()] = line.slice(at + 1).trim();
  }
  return { meta, body: raw.slice(match[0].length) };
}

function sideNav(nav, current) {
  const links = nav
    .map((item) => {
      const on = item.url === current ? ' aria-current="page"' : "";
      return `<li><a href="${base}${item.url}"${on}>${item.title}</a></li>`;
    })
    .join("\n          ");
  return `<nav class="doc-nav" aria-label="Documentation">
        <p class="eyebrow">Documentation</p>
        <ul>
          ${links}
        </ul>
      </nav>`;
}

function docShell(doc, nav) {
  const toc = doc.headings
    .map((h) => `<li><a href="#${h.id}">${h.text}</a></li>`)
    .join("\n          ");
  return `<div class="doc-grid">
      ${sideNav(nav, `docs/${doc.slug}/`)}
      <article class="prose">
        ${doc.html}
      </article>
      <aside class="doc-toc" aria-label="On this page" data-spy>
        <p class="eyebrow">On this page</p>
        <ul>
          ${toc}
        </ul>
      </aside>
    </div>`;
}

// --- Reference ------------------------------------------------------------

function referenceShell(reference, nav) {
  const toc = `<ul class="sub">
${reference.groups.map((group) => `          <li class="sub-group"><a href="#${group.id}">${escapeHtml(group.title)}</a>
            <ul>
${group.entries.map((entry) => `              <li><a href="#${entry.id}"><code>${escapeHtml(entry.title)}</code></a></li>`).join("\n")}
            </ul>
          </li>`).join("\n")}
        </ul>`;

  const body = reference.groups.map(referenceGroup).join("\n");

  return `<div class="doc-grid is-reference">
      ${sideNav(nav, "reference/")}
      <article class="prose reference">
        <h1>Reference</h1>
        <p class="lede">${escapeHtml(reference.intro)}</p>
        <p class="note"><span class="dot-soon" aria-hidden="true"></span> A dot marks a language that is not released yet.</p>
        ${body}
      </article>
      <aside class="doc-toc" aria-label="On this page" data-spy>
        <p class="eyebrow">On this page</p>
        ${toc}
      </aside>
    </div>`;
}

function referenceGroup(group) {
  const entries = group.entries.map(referenceEntry).join("\n");
  return `        <section class="ref-group" id="${group.id}">
          <h2>${escapeHtml(group.title)}</h2>
${entries}
        </section>`;
}

function referenceEntry(entry) {
  const available = LANGUAGES.filter((lang) => entry.examples[lang.id]);
  const tabs = available
    .map((lang, index) => `<button type="button" role="tab" data-lang="${lang.id}" aria-selected="${index === 0}">${lang.label}${lang.state === "soon" ? '<span class="dot-soon" aria-hidden="true"></span>' : ""}</button>`)
    .join("");
  const panes = available
    .map((lang, index) => {
      const planned = lang.state === "soon"
        ? `<p class="planned"><span class="dot-soon" aria-hidden="true"></span>Planned — the shape it will have.</p>`
        : "";
      return `<div class="pane" data-lang="${lang.id}"${index === 0 ? "" : " hidden"}>${planned}<pre><code>${escapeHtml(entry.examples[lang.id])}</code></pre></div>`;
    })
    .join("\n              ");
  const detail = entry.detail ? `<p>${marked.parseInline(entry.detail)}</p>` : "";
  return `          <article class="ref-entry" id="${entry.id}">
            <div class="ref-text">
              <h3><a class="anchor" href="#${entry.id}"><code>${escapeHtml(entry.title)}</code></a></h3>
              <p class="summary">${escapeHtml(entry.summary)}</p>
              ${detail}
            </div>
            <div class="langs" data-langs>
              <div class="tabs" role="tablist" aria-label="Language">${tabs}</div>
              ${panes}
            </div>
          </article>`;
}

// --- Devices --------------------------------------------------------------

async function readCatalog() {
  if (!existsSync(catalogPath)) {
    console.error(
      `missing ${relative(repo, catalogPath)}.\n` +
      `Generate it first: cargo run -p xtask -- catalog`,
    );
    process.exit(1);
  }
  return JSON.parse(await readFile(catalogPath, "utf8"));
}

const MODES = ["lan", "ble", "cloud"];
const CAPS = [
  ["power", "On / off"],
  ["brightness", "Brightness"],
  ["color", "Color"],
  ["colortemp", "White temperature"],
  ["segments", "Segments"],
  ["segment_brightness", "Segment brightness"],
  ["music", "Music"],
];

function renderDevices(template, catalog) {
  const devices = [...catalog.devices].sort((a, b) => a.sku.localeCompare(b.sku));
  return fill(template, {
    devices_rows: devices.map(deviceRow).join("\n"),
    devices_cards: devices.map(deviceCard).join("\n"),
  });
}

function deviceRow(d) {
  const cells = MODES.map((m) => {
    const support = d.modes?.[m]?.support ?? "unknown";
    return `<td><span class="pill pill-${support}">${support === "unknown" ? "?" : support}</span></td>`;
  }).join("");
  const date = d.verified?.date ?? "";
  const verified = date
    ? `<td class="verified"><span class="tick">verified</span> <time datetime="${date}">${date}</time></td>`
    : `<td class="verified"><span class="pill pill-unknown">?</span></td>`;
  const names = [d.sku, d.name, ...(d.aliases ?? [])].join(" ").toLowerCase();
  return `          <tr data-search="${escapeAttr(names)}" data-sku="${d.sku}">
            <th scope="row"><a href="#${d.sku}">${d.sku}</a></th>
            <td>${escapeHtml(d.name)}</td>${cells}${verified}
          </tr>`;
}

function deviceCard(d) {
  const caps = CAPS.filter(([key]) => d.capabilities && key in d.capabilities)
    .map(([, label]) => `<li>${label}</li>`)
    .join("");
  const modes = MODES.map((m) => {
    const mode = d.modes?.[m] ?? {};
    const support = mode.support ?? "unknown";
    const notes = mode.notes ? `<p>${escapeHtml(mode.notes)}</p>` : "";
    return `<div class="mode-block">
            <h4><code>${m}</code> <span class="pill pill-${support}">${support === "unknown" ? "?" : support}</span></h4>
            ${notes}
          </div>`;
  }).join("\n");
  const aliases = (d.aliases ?? []).length
    ? `<p class="note"><strong>Same device:</strong> ${d.aliases.join(", ")}.</p>`
    : "";
  const candidates = (d.candidate_aliases ?? []).length
    ? `<p class="note"><strong>Looks like the same product, not verified:</strong> ${d.candidate_aliases.join(", ")}. A different length has a different segment count.</p>`
    : "";
  const verified = d.verified?.date
    ? `<p class="note"><strong>Verified</strong> on ${d.verified.date}${d.verified.firmware ? `, firmware ${escapeHtml(d.verified.firmware)}` : ""}.</p>`
    : `<p class="note">Nobody has verified this model yet.</p>`;
  return `      <article class="device" id="${d.sku}" data-search="${escapeAttr([d.sku, d.name].join(" ").toLowerCase())}">
        <header>
          <h3>${d.sku}</h3>
          <p>${escapeHtml(d.name)}</p>
        </header>
        <div class="device-body">
          <div>
            <p class="eyebrow">What the hardware does</p>
            <ul class="caps">${caps}</ul>
            ${aliases}${candidates}${verified}
          </div>
          <div class="modes">
${modes}
          </div>
        </div>
        <footer><a href="{{repo}}/blob/main/devices/${d.sku}.yaml">Read the device file</a></footer>
      </article>`;
}

function escapeHtml(text) {
  return String(text).replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);
}

function escapeAttr(text) {
  return escapeHtml(text).replace(/"/g, "&quot;");
}


// --- Local server ----------------------------------------------------------

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".svg": "image/svg+xml",
  ".json": "application/json",
};

// A static server for `npm run dev`. It serves `dist/` and nothing else.
function serve() {
  const port = Number(process.env.PORT ?? 8787);
  createServer((request, response) => {
    let path = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
    // `npm run build` writes the same `dist/` with the published sub-path in
    // every link. The local server answers under that prefix as well, so a
    // production build does not leave the page without its stylesheet.
    if (path.startsWith(PAGES_BASE)) path = path.slice(PAGES_BASE.length - 1);
    let file = join(dist, path);
    if (!file.startsWith(dist)) return send(response, 403, "Forbidden");
    if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
    let ext = file.slice(file.lastIndexOf("."));
    if (!existsSync(file)) {
      file = join(dist, "404.html");
      if (!existsSync(file)) return send(response, 404, "Not found");
      // The page, not the type that was asked for: a missing stylesheet must
      // fail as a stylesheet and not arrive as HTML the browser parses.
      ext = ".html";
      response.statusCode = 404;
    }
    response.setHeader("Content-Type", TYPES[ext] ?? "application/octet-stream");
    response.setHeader("Cache-Control", "no-store");
    createReadStream(file).pipe(response);
  })
    // A second `npm run dev` on a port that is taken must say so. The default
    // is an unhandled error event, and a reader takes the stack trace for a
    // watcher that does not work: the first server keeps the page up.
    .on("error", (error) => {
      if (error.code !== "EADDRINUSE") throw error;
      console.error(`port ${port} is taken. Another server already serves the site.`);
      console.error(`Stop it with: kill $(lsof -ti :${port} -sTCP:LISTEN)`);
      console.error(`Or serve on another port: PORT=8788 npm run dev`);
      process.exit(1);
    })
    .listen(port, () => console.log(`http://localhost:${port}`));
}

function send(response, code, body) {
  response.statusCode = code;
  response.end(body);
}

// --- Watch ----------------------------------------------------------------

await main();

if (process.argv.includes("--serve")) serve();

if (process.argv.includes("--watch")) {
  let queued = null;
  for (const dir of ["src", "content"]) {
    watch(join(root, dir), { recursive: true }, () => {
      clearTimeout(queued);
      queued = setTimeout(() => main().catch(console.error), 80);
    });
  }
  console.log("watching src/ and content/ …");
}
