// Builds the static site into `dist/`.
//
// Three inputs, on purpose:
//   - `src/`     hand-written HTML, CSS and JS. Nothing generates it.
//   - `content/` Markdown pages, and `../dist/catalog.json` for the devices.
//   - `lib/`     the renderers this file calls.
//
// The catalog comes from the device files through `cargo run -p xtask --
// catalog`. The site never restates a device fact that the YAML carries.

import { cp, mkdir, readFile, readdir, rename, rm, writeFile } from "node:fs/promises";
import { createReadStream, existsSync, statSync, watch } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Marked } from "marked";
import { transform } from "esbuild";

import { escapeAttr, escapeHtml, fill, inline, slugify } from "./lib/html.mjs";
import { docShell, navItem } from "./lib/docs.mjs";
import { highlight } from "./lib/code.mjs";
import { MODES, devicePage, renderIndex, sorted } from "./lib/devices.mjs";
import { modeBadge } from "./lib/mode-badge.mjs";

const root = dirname(fileURLToPath(import.meta.url));
const repo = resolve(root, "..");
const dist = join(root, "dist");
// The build writes here and the result moves into `dist/` in one step. A
// reload during a rebuild then reaches the old site or the new one, never a
// directory that is half written. The name carries the process id, so a
// manual build beside a running `npm run dev` does not delete its staging
// directory under it.
const out = join(root, `.dist-build-${process.pid}`);

// The site answers on its own domain, from the root. Every link the pages
// carry is root-relative; the absolute form is for the canonical, the sitemap
// and the preview image, which a machine reads outside of a page.
const SITE_URL = "https://gvetk.com";
const base = "/";
const repoUrl = "https://github.com/damient/govee-toolkit";
const catalogPath = join(repo, "dist/catalog.json");
// The shape `lib/devices.mjs` reads. `xtask` writes it into every catalog.
const CATALOG_SCHEMA = 1;

const DESCRIPTION = "An unofficial toolkit that controls Govee lights over your own network, from Rust, Python, Node.js or the command line.";

const pages = [
  { src: "index.html", url: "", nav: "home", title: null, klass: "is-home" },
  {
    src: "devices.html",
    url: "devices/",
    nav: "devices",
    title: "Devices",
    render: renderIndex,
    description: "Which Govee models the toolkit reaches, over Wi-Fi, over Bluetooth and over the cloud. Built from the device files, so it cannot disagree with them.",
  },
];

// The reference page sits inside the documentation menu, between the two
// Markdown pages that surround it.
const REFERENCE = { url: "reference/", title: "Reference", order: 4 };

// Pages that the footer does not name.
const FOOT_SKIP = new Set(["docs/configure/", "docs/troubleshooting/"]);

// `planned` marks a package that has no code yet: its example shows the shape
// the call will have, and the pane says so.
const LANGUAGES = [
  { id: "cli", label: "Command line" },
  { id: "rust", label: "Rust" },
  { id: "python", label: "Python", planned: true },
  { id: "node", label: "Node.js", planned: true },
];

async function main() {
  const catalog = await readCatalog();
  await rm(out, { recursive: true, force: true });
  await mkdir(out, { recursive: true });

  // `assets()` writes the stylesheet into the pages and the minified script
  // into `assets/js`, so neither source is copied here.
  const skip = join(root, "src/assets/css");
  const copy = {
    recursive: true,
    filter: (path) => !path.endsWith(".DS_Store") && !path.startsWith(skip),
  };
  await cp(join(root, "src/assets"), join(out, "assets"), copy);
  if (existsSync(join(root, "public"))) {
    await cp(join(root, "public"), out, { recursive: true, filter: (path) => !path.endsWith(".DS_Store") });
  }

  const [, css, layout, docs, reference] = await Promise.all([
    writeFile(join(out, ".nojekyll"), ""),
    assets(),
    readFile(join(root, "src/layout.html"), "utf8"),
    readDocs(),
    readFile(join(root, "content/reference.json"), "utf8").then(JSON.parse),
  ]);
  const nav = [
    ...docs.map((d) => ({ url: `docs/${d.slug}/`, title: d.title, order: d.order })),
    REFERENCE,
  ].sort((a, b) => a.order - b.order);
  // The docs entry point is the first page in the menu, not a hardcoded slug.
  const docsHome = nav[0].url;
  // The footer names a short list. The side menu keeps every page.
  const docNav = nav
    .filter((item) => !FOOT_SKIP.has(item.url))
    .map((item) => navItem(base, item, null))
    .join("\n          ");
  // `vars` is built once: every page fills its body with the same values.
  const ctx = { css, docsHome, docNav, vars: { base, repo: repoUrl, ...modeBadges() } };

  const devices = sorted(catalog);
  const sources = await Promise.all(
    pages.map((page) => readFile(join(root, "src/pages", page.src), "utf8")),
  );

  // One record per page, so a new field is added here and read in one place.
  const all = [
    ...pages.map((page, at) => ({
      ...page,
      jsonld: page.url === "" ? homeData() : [],
      body: page.render ? page.render(sources[at], devices) : sources[at],
    })),
    ...devices.map((device) => {
      const page = devicePage(device);
      return {
        url: page.url,
        nav: "devices",
        title: page.title,
        description: page.description,
        klass: "is-device",
        jsonld: [breadcrumb(page.breadcrumb), deviceData(device, page)],
        body: page.body,
      };
    }),
    ...docs.map((doc) => ({
      url: `docs/${doc.slug}/`,
      nav: "docs",
      title: doc.title,
      description: doc.description,
      klass: "is-doc",
      jsonld: [
        breadcrumb([["Docs", docsHome], [doc.title, `docs/${doc.slug}/`]]),
        ...(doc.faq ? [faqData(doc)] : []),
      ],
      body: docPage(doc, nav),
    })),
    {
      url: REFERENCE.url,
      nav: "docs",
      title: REFERENCE.title,
      description: reference.intro,
      klass: "is-doc",
      jsonld: [breadcrumb([["Docs", docsHome], ["Reference", "reference/"]])],
      body: referencePage(reference, nav),
    },
    {
      url: "404.html",
      nav: "",
      title: "Page not found",
      noindex: true,
      body: `<section class="slab"><h1>Page not found</h1><p class="lede">That page does not exist. <a href="${base}">Go back to the start</a>.</p></section>`,
    },
  ];

  // The pages do not depend on each other, so they are written together.
  // `Promise.all` keeps the order, and the sitemap follows it.
  const written = await Promise.all(all.map((page) => emit(layout, page, ctx)));
  const sitemap = written.filter(Boolean);

  await writeFile(join(out, "robots.txt"), robots());
  await writeFile(join(out, "sitemap.xml"), sitemapXml(sitemap));

  await publish();
  const clock = new Date().toTimeString().slice(0, 8);
  console.log(`${clock}  site -> ${relative(repo, dist)} (${sitemap.length} pages, ${devices.length} devices)`);
}

// The stylesheet is inlined into every page, so the copied one would be dead
// weight. The script stays a file: it is deferred, and a second page reads it
// from the cache.
async function assets() {
  const [source, script] = await Promise.all([
    readFile(join(root, "src/assets/css/site.css"), "utf8"),
    readFile(join(root, "src/assets/js/site.js"), "utf8"),
  ]);
  const [css, min] = await Promise.all([
    transform(source, { loader: "css", minify: true }),
    transform(script, { loader: "js", minify: true, target: "es2022" }),
  ]);
  await mkdir(join(out, "assets/js"), { recursive: true });
  await writeFile(join(out, "assets/js/site.js"), min.code);
  return css.code;
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

/** Writes one page and returns its canonical URL, or `null` when `noindex`. */
async function emit(layout, page, ctx) {
  const canonical = `${SITE_URL}${base}${page.url}`;
  const html = fill(layout, {
    base,
    site: SITE_URL,
    repo: repoUrl,
    lang: "en",
    title: page.title ? `${page.title} — Govee Toolkit` : "Govee Toolkit — control your Govee lights locally",
    description: page.description ?? DESCRIPTION,
    bodyclass: page.klass ?? "",
    canonical,
    og_type: page.url === "" ? "website" : "article",
    robots: page.noindex
      ? '<meta name="robots" content="noindex, follow">'
      : '<meta name="robots" content="index, follow, max-image-preview:large">',
    css: ctx.css,
    jsonld: jsonLd(page.jsonld),
    nav: topNav(page.nav, ctx.docsHome),
    docnav: ctx.docNav,
    content: fill(page.body, ctx.vars),
    year: String(new Date().getUTCFullYear()),
  });
  const file = page.url.endsWith(".html")
    ? join(out, page.url)
    : join(out, page.url, "index.html");
  await mkdir(dirname(file), { recursive: true });
  await writeFile(file, html);
  return page.noindex ? null : canonical;
}

// `{{badge_lan}}` and friends, so a static page names a mode with the same
// component the model pages use.
function modeBadges() {
  return Object.fromEntries(MODES.map((m) => [`badge_${m}`, modeBadge(m)]));
}


// The reference page sits inside the documentation, so `key` marks Docs while
// the reader is on it. One entry carries the three facts a link needs.
function topNav(current, docsHome) {
  const items = [
    { url: "", key: "home", label: "Home" },
    { url: docsHome, key: "docs", label: "Docs" },
    { url: "devices/", key: "devices", label: "Devices" },
  ];
  return items
    .map(({ url, key, label }) => {
      const on = key === current ? ' aria-current="page"' : "";
      return `<a href="${base}${url}"${on}>${label}</a>`;
    })
    .join("\n        ");
}

// --- What a machine reads --------------------------------------------------

function jsonLd(blocks) {
  if (!blocks?.length) return "";
  return blocks
    .map((block) => `<script type="application/ld+json">${JSON.stringify(block)}</script>`)
    .join("\n");
}

function homeData() {
  return [
    {
      "@context": "https://schema.org",
      "@type": "WebSite",
      name: "Govee Toolkit",
      url: `${SITE_URL}${base}`,
      description: DESCRIPTION,
      inLanguage: "en",
    },
    {
      "@context": "https://schema.org",
      "@type": "SoftwareSourceCode",
      name: "govee-toolkit",
      description: DESCRIPTION,
      codeRepository: repoUrl,
      programmingLanguage: ["Rust", "Python", "JavaScript"],
      license: "https://opensource.org/licenses/MIT",
      url: `${SITE_URL}${base}`,
    },
  ];
}

function breadcrumb(trail) {
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [["Home", ""], ...trail].map(([name, url], index) => ({
      "@type": "ListItem",
      position: index + 1,
      name,
      item: `${SITE_URL}${base}${url}`,
    })),
  };
}

function deviceData(device, page) {
  return {
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline: page.title,
    description: page.description,
    url: `${SITE_URL}${base}${page.url}`,
    inLanguage: "en",
    isPartOf: { "@type": "WebSite", name: "Govee Toolkit", url: `${SITE_URL}${base}` },
    ...(device.verified?.date ? { dateModified: device.verified.date } : {}),
  };
}

// The questions are the second-level headings, and the answer is the section
// each one opens. A page that declares no `faq` in its front matter gets none.
function faqData(doc) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: doc.sections.map((section) => ({
      "@type": "Question",
      name: section.title,
      acceptedAnswer: { "@type": "Answer", text: section.answer },
    })),
  };
}

function robots() {
  return `User-agent: *\nAllow: /\n\nSitemap: ${SITE_URL}${base}sitemap.xml\n`;
}

// No `lastmod`: the build date is the date of the build and not the date the
// page changed, and a wrong one is worse than none.
function sitemapXml(sitemap) {
  const urls = sitemap
    .map((url) => `  <url><loc>${escapeHtml(url)}</loc></url>`)
    .join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;
}

// --- Markdown -------------------------------------------------------------

async function readDocs() {
  const dir = join(root, "content/docs");
  const files = (await readdir(dir)).filter((f) => f.endsWith(".md"));
  const raw = await Promise.all(files.map((f) => readFile(join(dir, f), "utf8")));
  const docs = files.map((file, at) => renderDoc(file, raw[at]));
  return docs.sort((a, b) => a.order - b.order);
}

function renderDoc(file, raw) {
  const { meta, body } = frontMatter(raw);
  // The heading renderer fills this: the id it writes into the anchor is the
  // id the table of contents links to, derived once.
  const headings = [];
  const md = new Marked({ async: false });
  md.use({
    renderer: {
      code({ text, lang }) {
        const name = (lang ?? "").trim().split(/\s+/)[0];
        return `<pre><code>${highlight(text, name)}</code></pre>\n`;
      },
      heading({ depth, tokens }) {
        const inner = this.parser.parseInline(tokens);
        if (depth !== 2) return `<h${depth}>${inner}</h${depth}>\n`;
        const label = headingLabel(tokens);
        const id = slugify(label);
        headings.push({ id, text: label });
        return `<h2 id="${escapeAttr(id)}"><a class="anchor" href="#${escapeAttr(id)}">${inner}</a></h2>\n`;
      },
    },
  });
  const html = md.parse(fill(body, { base, repo: repoUrl, ...modeBadges() }));
  return {
    slug: meta.slug ?? file.replace(/\.md$/, ""),
    title: meta.title ?? file,
    description: meta.description ?? "",
    order: Number(meta.order ?? 99),
    faq: meta.faq === "true",
    headings,
    html,
    sections: sections(html, headings),
  };
}

// Cuts the rendered page at every second-level heading. The answer is the
// text under the heading, which is what the markup must match: the heading
// itself is the question and must not repeat inside its own answer.
function sections(html, headings) {
  const parts = html.split(/<h2 id="[^"]*">/).slice(1);
  return headings.map((heading, index) => {
    const part = parts[index] ?? "";
    const close = part.indexOf("</h2>");
    return {
      title: heading.text,
      answer: text(close === -1 ? part : part.slice(close + 5)),
    };
  });
}

function text(html) {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">")
    .replace(/&#39;|&quot;/g, '"')
    .replace(/\s+/g, " ")
    .trim();
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

function docPage(doc, nav) {
  return docShell({
    base,
    nav,
    current: `docs/${doc.slug}/`,
    toc: doc.headings.map((h) => ({ id: h.id, text: h.text })),
    body: doc.html,
  });
}

// --- Reference ------------------------------------------------------------

function referencePage(reference, nav) {
  const toc = reference.groups.map((group) => ({
    id: group.id,
    text: group.title,
    children: group.entries.map((entry) => ({ id: entry.id, text: entry.title, code: true })),
  }));

  return docShell({
    base,
    nav,
    current: REFERENCE.url,
    toc,
    klass: "reference",
    body: `<h1>Reference</h1>
        <p class="lede">${escapeHtml(reference.intro)}</p>
${reference.groups.map(referenceGroup).join("\n")}`,
  });
}

function referenceGroup(group) {
  const entries = group.entries.map(referenceEntry).join("\n");
  return `        <section class="ref-group">
          <h2 id="${escapeAttr(group.id)}"><a class="anchor" href="#${escapeAttr(group.id)}">${escapeHtml(group.title)}</a></h2>
${entries}
        </section>`;
}

// Each tab names the panel it controls, and each panel names its tab: a reader
// who arrives on a panel with a screen reader is told which language it is.
function referenceEntry(entry) {
  const available = LANGUAGES.filter((lang) => entry.examples[lang.id]);
  const id = (lang) => `${entry.id}-${lang.id}`;
  const tabs = available
    .map((lang, index) => `<button type="button" role="tab" id="tab-${id(lang)}" aria-controls="pane-${id(lang)}" data-lang="${lang.id}" aria-selected="${index === 0}" tabindex="${index === 0 ? "0" : "-1"}">${lang.label}</button>`)
    .join("");
  const panes = available
    .map((lang, index) => {
      const planned = lang.planned
        ? `<p class="planned">Planned — the shape it will have.</p>`
        : "";
      const source = entry.examples[lang.id];
      return `<div class="pane" role="tabpanel" id="pane-${id(lang)}" aria-labelledby="tab-${id(lang)}" tabindex="0" data-lang="${lang.id}"${index === 0 ? "" : " hidden"}>${planned}<pre><code>${highlight(source, lang.id)}</code></pre><button class="copy" type="button" data-copy="${escapeAttr(source)}">Copy</button></div>`;
    })
    .join("\n              ");
  const detail = entry.detail ? `<p>${inline(entry.detail)}</p>` : "";
  return `          <article class="ref-entry" id="${escapeAttr(entry.id)}">
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
  const catalog = JSON.parse(await readFile(catalogPath, "utf8"));
  if (catalog.schema_version !== CATALOG_SCHEMA) {
    console.error(
      `${relative(repo, catalogPath)} is schema ${catalog.schema_version}, ` +
      `and the renderers read schema ${CATALOG_SCHEMA}.\n` +
      `A field that moved renders as "?" and as "not verified", which states ` +
      `something nobody established. Update lib/devices.mjs first.`,
    );
    process.exit(1);
  }
  return catalog;
}

// --- Local server ----------------------------------------------------------

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
  ".json": "application/json",
  ".xml": "application/xml",
  ".txt": "text/plain; charset=utf-8",
};

// A static server for `npm run dev`. It serves `dist/` and nothing else.
function serve() {
  const port = Number(process.env.PORT ?? 8787);
  createServer((request, response) => {
    const path = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
    let file = join(dist, path);
    if (!file.startsWith(dist)) return send(response, 403, "Forbidden");
    if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
    let ext = extname(file);
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
  for (const dir of ["src", "content", "lib"]) {
    watch(join(root, dir), { recursive: true }, () => {
      clearTimeout(queued);
      queued = setTimeout(() => main().catch(console.error), 80);
    });
  }
  console.log("watching src/, content/ and lib/ …");
}
