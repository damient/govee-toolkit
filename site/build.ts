// Builds the static site into `dist/`. The inputs are in `README.md`. The site
// never restates a device fact that the YAML carries.

import { cp, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { assets } from "./lib/assets.ts";
import { DESCRIPTION, SITE_URL, base, dist, repo, repoUrl, root } from "./lib/config.ts";
import { type Doc, docPage, readDocs } from "./lib/content.ts";
import { sweepStaging, watchSources } from "./lib/dev.ts";
import { crumbs } from "./lib/crumbs.ts";
import { devicePage, renderIndex, sorted } from "./lib/devices.ts";
import { navItem } from "./lib/docs.ts";
import { faqPage } from "./lib/faq.ts";
import { readLanList, renderLanList } from "./lib/lan-list.ts";
import { fill } from "./lib/html.ts";
import { isReference, readCatalog, readJson } from "./lib/json.ts";
import { dmxBadge, modeBadges } from "./lib/mode-badge.ts";
import { REFERENCE, referencePage } from "./lib/reference.ts";
import { breadcrumb, deviceData, homeData, jsonLd, robots, sitemapXml } from "./lib/seo.ts";
import { type Change, notify, serve } from "./lib/serve.ts";
import type { Crumb, Device, LanList, NavEntry, Reference } from "./lib/types.ts";

/** A hand-written page under `src/pages/`. */
interface Source {
  src: string;
  url: string;
  nav: string;
  title: string | null;
  klass?: string;
  render?: (template: string, devices: Device[], lan: LanList) => string;
  trail?: Crumb[];
  description?: string;
}

/** One page, ready for the layout. */
interface Page {
  url: string;
  nav: string;
  title: string | null;
  description?: string;
  klass?: string;
  noindex?: boolean;
  jsonld?: Record<string, unknown>[];
  body: string;
}

interface Context {
  css: string;
  docsHome: string;
  docNav: string;
  vars: Record<string, string>;
}

// A reload during a rebuild reaches the old site or the new one, never half of
// one. The process id keeps a manual build off the staging of `npm run dev`.
const out = join(root, `.dist-build-${process.pid}`);

const pages: Source[] = [
  { src: "index.html", url: "", nav: "home", title: null, klass: "is-home" },
  {
    src: "devices.html",
    url: "devices/",
    nav: "devices",
    title: "Devices",
    render: renderIndex,
    trail: [["Devices", "devices/"]],
    description: "Which Govee lights work with Govee Toolkit: what each model answers over Wi-Fi, over Bluetooth, through the cloud and from a DMX desk.",
  },
  {
    src: "add-device.html",
    url: "devices/add/",
    nav: "devices",
    title: "Add a device",
    klass: "is-add",
    trail: [["Devices", "devices/"], ["Add a device", "devices/add/"]],
    description: "Two ways to put your Govee light on this list: send it to us and we add it for free, or follow the guide and add it yourself.",
  },
  {
    src: "lan-devices.html",
    url: "devices/lan/",
    nav: "devices",
    title: "Models with LAN Control",
    render: renderLanList,
    trail: [["Devices", "devices/"], ["LAN Control", "devices/lan/"]],
    description: "Every Govee model that carries the LAN Control switch, which lets your own network reach the light over Wi-Fi. Search by SKU or by name.",
  },
];

const FOOT_SKIP = new Set(["docs/configure/", "docs/troubleshooting/"]);

async function main(): Promise<void> {
  const catalog = await readCatalog();
  await stage();
  const [, css, layout, docs, reference, faq, lan] = await Promise.all([
    writeFile(join(out, ".nojekyll"), ""),
    assets(out),
    readFile(join(root, "src/layout.html"), "utf8"),
    readDocs(),
    readJson(join(root, "content/reference.json"), isReference),
    faqPage(),
    readLanList(),
  ]);
  const nav: NavEntry[] = [
    ...docs.map((d) => ({ url: `docs/${d.slug}/`, title: d.title, order: d.order })),
    REFERENCE,
  ].toSorted((a, b) => a.order - b.order);
  const docsHome = nav[0]?.url ?? REFERENCE.url;
  // The footer names a short list. The side menu keeps every page.
  const docNav = nav
    .filter((item) => !FOOT_SKIP.has(item.url))
    .map((item) => navItem(base, item, null))
    .join("\n          ");
  const ctx: Context = { css, docsHome, docNav, vars: { base, repo: repoUrl, badge_dmx: dmxBadge(), ...modeBadges() } };

  const devices = sorted(catalog);
  const all: Page[] = [
    ...(await sourcePages(devices, lan)),
    ...devices.map((device) => deviceEntry(device, reference)),
    ...docPages(docs, nav, docsHome, reference),
    faq,
    NOT_FOUND,
  ];

  const written = await Promise.all(all.map((page) => emit(layout, page, ctx)));
  const sitemap = written.filter((url) => url !== null);

  await writeFile(join(out, "robots.txt"), robots());
  await writeFile(join(out, "sitemap.xml"), sitemapXml(sitemap));

  await publish();
  const clock = new Date().toTimeString().slice(0, 8);
  console.log(`${clock}  site -> ${relative(repo, dist)} (${sitemap.length} pages, ${devices.length} devices)`);
}

// `assets()` bundles the stylesheet and the script: only the bundle is copied.
async function stage(): Promise<void> {
  await rm(out, { recursive: true, force: true });
  await mkdir(out, { recursive: true });
  const skip = [join(root, "src/assets/css"), join(root, "src/assets/ts")];
  const copy = {
    recursive: true,
    filter: (path: string) => !path.endsWith(".DS_Store") && !skip.some((dir) => path.startsWith(dir)),
  };
  await cp(join(root, "src/assets"), join(out, "assets"), copy);
  if (existsSync(join(root, "public"))) {
    await cp(join(root, "public"), out, { recursive: true, filter: (path: string) => !path.endsWith(".DS_Store") });
  }
}

function sourcePages(devices: Device[], lan: LanList): Promise<Page[]> {
  return Promise.all(pages.map(async (page) => {
    const source = await readFile(join(root, "src/pages", page.src), "utf8");
    const jsonld = page.url === "" ? homeData() : page.trail ? [breadcrumb(page.trail)] : [];
    const body = fill(page.render ? page.render(source, devices, lan) : source, {
      crumbs: page.trail ? crumbs(page.trail) : "",
    });
    return Object.assign({ jsonld, body }, page);
  }));
}

function deviceEntry(device: Device, reference: Reference): Page {
  const page = devicePage(device, reference);
  return {
    url: page.url,
    nav: "devices",
    title: page.title,
    description: page.description,
    klass: "is-device",
    jsonld: [breadcrumb(page.breadcrumb), deviceData(device, page)],
    body: page.body,
  };
}

function docPages(docs: Doc[], nav: NavEntry[], docsHome: string, reference: Reference): Page[] {
  return [
    ...docs.map((doc) => ({
      url: `docs/${doc.slug}/`,
      nav: "docs",
      title: doc.title,
      description: doc.description,
      klass: "is-doc",
      jsonld: [breadcrumb([["Docs", docsHome], [doc.title, `docs/${doc.slug}/`]])],
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
  ];
}

const NOT_FOUND: Page = {
  url: "404.html",
  nav: "",
  title: "Page not found",
  noindex: true,
  body: `<section class="slab"><h1>Page not found</h1><p class="lede">That page does not exist. <a href="${base}">Go back to the start</a>.</p></section>`,
};

// Two renames: `dist/` is missing for microseconds, not for a whole build.
async function publish(): Promise<void> {
  const previous = `${dist}.previous`;
  await rm(previous, { recursive: true, force: true });
  if (existsSync(dist)) await rename(dist, previous);
  await rename(out, dist);
  await rm(previous, { recursive: true, force: true });
}

/** Writes one page and returns its canonical URL, or `null` when `noindex`. */
async function emit(layout: string, page: Page, ctx: Context): Promise<string | null> {
  const canonical = `${SITE_URL}${base}${page.url}`;
  const html = fill(layout, {
    base,
    site: SITE_URL,
    repo: repoUrl,
    lang: "en",
    title: page.title !== null && page.title !== "" ? `${page.title} — Govee Toolkit` : "Govee Toolkit — control your Govee lights locally",
    description: page.description ?? DESCRIPTION,
    bodyclass: page.klass ?? "",
    canonical,
    og_type: page.url === "" ? "website" : "article",
    robots: page.noindex === true
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
  return page.noindex === true ? null : canonical;
}

function topNav(current: string, docsHome: string): string {
  const items = [
    { url: "", key: "home", label: "Home" },
    { url: docsHome, key: "docs", label: "Docs" },
    { url: "devices/", key: "devices", label: "Devices" },
    { url: "faq/", key: "faq", label: "FAQ" },
  ];
  return items
    .map(({ url, key, label }) => {
      const on = key === current ? ' aria-current="page"' : "";
      return `<a href="${base}${url}"${on}>${label}</a>`;
    })
    .join("\n        ");
}

// `npm run dev`: a failed build leaves the server and the watch up.
const dev = process.argv.includes("--dev");

if (dev) {
  await sweepStaging();
  await main().catch(console.error);
} else {
  await main();
}

if (dev || process.argv.includes("--serve")) serve({ live: dev });

if (dev) {
  watchSources(async (change: Change) => {
    await main();
    notify(change);
  });
}
