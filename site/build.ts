// Builds the static site into `dist/`. See `README.md` for the inputs, and
// `lib/config.ts` for the address of the site.
//
// The catalog comes from the device files through `cargo run -p xtask --
// catalog`. The site never restates a device fact that the YAML carries.

import { cp, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { assets } from "./lib/assets.ts";
import { CATALOG_SCHEMA, DESCRIPTION, SITE_URL, base, catalogPath, dist, repo, repoUrl, root } from "./lib/config.ts";
import { docPage, readDocs } from "./lib/content.ts";
import { sweepStaging, watchSources } from "./lib/dev.ts";
import { crumbs } from "./lib/crumbs.ts";
import { devicePage, renderIndex, sorted } from "./lib/devices.ts";
import { navItem } from "./lib/docs.ts";
import { fill } from "./lib/html.ts";
import { dmxBadge, modeBadges } from "./lib/mode-badge.ts";
import { REFERENCE, referencePage } from "./lib/reference.ts";
import { breadcrumb, deviceData, faqData, homeData, jsonLd, robots, sitemapXml } from "./lib/seo.ts";
import { type Change, notify, serve } from "./lib/serve.ts";
import type { Catalog, Crumb, Device, NavEntry, Reference } from "./lib/types.ts";

/** A hand-written page under `src/pages/`. */
interface Source {
  src: string;
  url: string;
  nav: string;
  title: string | null;
  klass?: string;
  render?: (template: string, devices: Device[]) => string;
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

// Staging, so that a reload during a rebuild reaches the old site or the new
// one and never a half-written directory. The process id keeps a manual build
// from deleting the staging directory of a running `npm run dev`.
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
    description: "Which Govee models the toolkit reaches, over Wi-Fi, over Bluetooth, over the cloud and from a DMX desk. Built from the device files, so it cannot disagree with them.",
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
];

const FOOT_SKIP = new Set(["docs/configure/", "docs/troubleshooting/"]);

async function main(): Promise<void> {
  const catalog = await readCatalog();
  await rm(out, { recursive: true, force: true });
  await mkdir(out, { recursive: true });

  // `assets()` bundles the stylesheet and the script, so neither source
  // directory is copied: only the bundle reaches the site.
  const skip = [join(root, "src/assets/css"), join(root, "src/assets/ts")];
  const copy = {
    recursive: true,
    filter: (path: string) => !path.endsWith(".DS_Store") && !skip.some((dir) => path.startsWith(dir)),
  };
  await cp(join(root, "src/assets"), join(out, "assets"), copy);
  if (existsSync(join(root, "public"))) {
    await cp(join(root, "public"), out, { recursive: true, filter: (path: string) => !path.endsWith(".DS_Store") });
  }

  const [, css, layout, docs, reference] = await Promise.all([
    writeFile(join(out, ".nojekyll"), ""),
    assets(out),
    readFile(join(root, "src/layout.html"), "utf8"),
    readDocs(),
    readFile(join(root, "content/reference.json"), "utf8").then((text): Reference => JSON.parse(text)),
  ]);
  const nav: NavEntry[] = [
    ...docs.map((d) => ({ url: `docs/${d.slug}/`, title: d.title, order: d.order })),
    REFERENCE,
  ].sort((a, b) => a.order - b.order);
  const docsHome = nav[0]?.url ?? REFERENCE.url;
  // The footer names a short list. The side menu keeps every page.
  const docNav = nav
    .filter((item) => !FOOT_SKIP.has(item.url))
    .map((item) => navItem(base, item, null))
    .join("\n          ");
  const ctx: Context = { css, docsHome, docNav, vars: { base, repo: repoUrl, badge_dmx: dmxBadge(), ...modeBadges() } };

  const devices = sorted(catalog);
  const sources = await Promise.all(
    pages.map((page) => readFile(join(root, "src/pages", page.src), "utf8")),
  );

  const all: Page[] = [
    ...pages.map((page, at) => ({
      ...page,
      jsonld: page.url === "" ? homeData() : page.trail ? [breadcrumb(page.trail)] : [],
      body: fill(page.render ? page.render(sources[at] ?? "", devices) : (sources[at] ?? ""), {
        crumbs: page.trail ? crumbs(page.trail) : "",
      }),
    })),
    ...devices.map((device) => {
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

  // `Promise.all` keeps the order, and the sitemap follows it.
  const written = await Promise.all(all.map((page) => emit(layout, page, ctx)));
  const sitemap = written.filter((url) => url !== null);

  await writeFile(join(out, "robots.txt"), robots());
  await writeFile(join(out, "sitemap.xml"), sitemapXml(sitemap));

  await publish();
  const clock = new Date().toTimeString().slice(0, 8);
  console.log(`${clock}  site -> ${relative(repo, dist)} (${sitemap.length} pages, ${devices.length} devices)`);
}

// Two renames, so that `dist/` is missing for microseconds instead of for the
// length of a build.
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

// The reference page sits inside the documentation, so it marks Docs while the
// reader is on it.
function topNav(current: string, docsHome: string): string {
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

async function readCatalog(): Promise<Catalog> {
  if (!existsSync(catalogPath)) {
    console.error(
      `missing ${relative(repo, catalogPath)}.\n` +
      `Generate it first: cargo run -p xtask -- catalog`,
    );
    process.exit(1);
  }
  const catalog: Catalog = JSON.parse(await readFile(catalogPath, "utf8"));
  if (catalog.schema_version !== CATALOG_SCHEMA) {
    console.error(
      `${relative(repo, catalogPath)} is schema ${catalog.schema_version}, ` +
      `and the renderers read schema ${CATALOG_SCHEMA}.\n` +
      `A field that moved renders as "?" and as "not verified", which states ` +
      `something nobody established. Update lib/types.ts and lib/devices.ts first.`,
    );
    process.exit(1);
  }
  return catalog;
}

// `--dev` is what `npm run dev` passes: a failed build leaves the server and
// the watch up, so the next save can fix it.
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
