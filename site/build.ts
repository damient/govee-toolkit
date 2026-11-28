// Builds the static site into `dist/`. The inputs are in `README.md`. The site
// never restates a device fact that the YAML carries.

import { cp, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, relative } from "node:path";
import { assets } from "./lib/assets.ts";
import { base, dist, repo, repoUrl, root } from "./lib/config.ts";
import { type Doc, docPage, readDocs } from "./lib/content.ts";
import { sweepStaging, watchSources } from "./lib/dev.ts";
import { categoryPages } from "./lib/categories.ts";
import { crumbs } from "./lib/crumbs.ts";
import { devicePage } from "./lib/device-page.ts";
import { renderIndex } from "./lib/devices.ts";
import { type Draft, type Frame, type Page, address, emit } from "./lib/emit.ts";
import { everyModel } from "./lib/listed.ts";
import { faqPage } from "./lib/faq.ts";
import { fill } from "./lib/html.ts";
import { isReference, readApi, readCatalog, readCategories, readJson } from "./lib/json.ts";
import { ENGLISH, LOCALES, type Locale, type T, forgetStrings, strings, translate } from "./lib/locales.ts";
import { dmxBadge, modeBadges } from "./lib/mode-badge.ts";
import { productSources } from "./lib/products.ts";
import { REFERENCE_URL, localReference, referenceNav, referencePage } from "./lib/reference.ts";
import { breadcrumb, deviceData, homeData, redirect, robots, sitemapXml } from "./lib/seo.ts";
import { type Change, notify, serve } from "./lib/serve.ts";
import type { Api, Crumb, Device, NavEntry, Reference } from "./lib/types.ts";

/** A hand-written page under `src/pages/`. */
interface Source {
  src: string;
  url: string;
  nav: string;
  title: string | null;
  klass?: string;
  noindex?: boolean;
  render?: (template: string, devices: Device[]) => string;
  trail?: Crumb[];
  description?: string;
  jsonld?: Record<string, unknown>[];
}

interface Shared {
  layout: string;
  devices: Device[];
  reference: Reference;
  api: Api;
  css: string;
}

// A reload during a rebuild reaches the old site or the new one, never half of
// one. The process id keeps a manual build off the staging of `npm run dev`.
const out = join(root, `.dist-build-${process.pid}`);

function sources(t: T): Source[] {
  return [
    {
      src: "devices.html",
      url: "devices/",
      nav: "devices",
      title: t("page.devices"),
      render: (template, devices) => renderIndex(template, devices, { kind: "all" }, t),
      trail: [[t("page.devices"), "devices/"]],
      description: t("page.devices_description"),
    },
    {
      src: "add-device.html",
      url: "devices/add/",
      nav: "devices",
      title: t("page.add"),
      klass: "is-add",
      trail: [[t("page.devices"), "devices/"], [t("page.add"), "devices/add/"]],
      description: t("page.add_description"),
    },
    ...productSources(t),
  ];
}

async function main(): Promise<void> {
  forgetStrings();
  const [catalog, api, categories] = await Promise.all([readCatalog(), readApi(), readCategories()]);
  await stage();
  const [, css, layout, reference] = await Promise.all([
    writeFile(join(out, ".nojekyll"), ""),
    assets(out),
    readFile(join(root, "src/layout.html"), "utf8"),
    readJson(join(root, "content/reference.json"), isReference),
  ]);
  const shared: Shared = { layout, devices: everyModel(catalog, categories), reference, api, css };
  const built = await Promise.all(LOCALES.map((locale) => localePages(locale, shared)));
  const missing = notFound();
  const all: Page[] = [...built.flatMap((b) => b.pages), missing];

  unique([...all.map((page) => address(page)), ...Object.keys(MOVED)]);
  const paths = (keep: (page: Page) => boolean) => new Map(LOCALES.map((locale) => [
    locale.id,
    new Set(all.filter((p) => p.locale.id === locale.id && keep(p)).map((p) => p.url)),
  ]));
  const maps = { routes: paths((p) => p !== missing), site: paths((p) => p.noindex !== true) };
  const english = built[0]?.frame;
  if (!english) throw new Error("no English frame");
  await Promise.all([
    ...built.flatMap(({ frame, pages }) => pages.map((page) => emit(out, page, frame, maps))),
    emit(out, missing, english, maps),
  ]);

  await Promise.all(Object.entries(MOVED).map(async ([from, to]) => {
    await mkdir(join(out, from), { recursive: true });
    await writeFile(join(out, from, "index.html"), redirect(to));
  }));
  await writeFile(join(out, "robots.txt"), robots());
  await writeFile(join(out, "sitemap.xml"), sitemapXml(maps.site));

  await publish();
  const clock = new Date().toTimeString().slice(0, 8);
  const indexed = [...maps.site.values()].reduce((n, urls) => n + urls.size, 0);
  console.log(`${clock}  site -> ${relative(repo, dist)} (${indexed} pages, ${shared.devices.length} devices, ${LOCALES.length} languages)`);
}

async function localePages(locale: Locale, { layout, devices, reference, api, css }: Shared) {
  const t = strings(locale);
  const [docs, faq] = await Promise.all([readDocs(locale), faqPage(locale)]);
  const ref = localReference(reference, locale);
  const nav: NavEntry[] = [
    ...docs.map((d) => ({ url: `docs/${d.slug}/`, title: d.title, order: d.order })),
    ...(ref === null ? [] : [referenceNav(t)]),
  ].toSorted((a, b) => a.order - b.order);
  const docsHome = nav[0]?.url ?? REFERENCE_URL;
  const frame: Frame = { layout: translate(layout, t), css, docsHome, vars: { base, repo: repoUrl, badge_dmx: dmxBadge(), ...modeBadges() } };
  const pages: Draft[] = [
    ...(await sourcePages([...sources(t), ...categoryPages(devices, renderIndex, locale)], devices, locale)),
    ...docPages(docs, nav, docsHome, locale),
    ...(ref === null ? [] : [refPage(ref, api, nav, docsHome, locale)]),
    ...(faq === null ? [] : [faq]),
    ...devices.map((device) => deviceEntry(device, ref ?? reference, devices, locale)),
  ];
  return { frame, pages: pages.map((page): Page => Object.assign({}, page, { locale })) };
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

// A file system can ignore case, so two URLs that differ in case collide.
function unique(urls: string[]): void {
  const seen = new Set<string>();
  for (const url of urls) {
    const key = url.toLowerCase();
    if (seen.has(key)) throw new Error(`two pages take ${url}`);
    seen.add(key);
  }
}

// Every category page shares the template of the devices page.
function sourcePages(list: Source[], devices: Device[], locale: Locale): Promise<Draft[]> {
  const t = strings(locale);
  const templates = new Map<string, Promise<string>>();
  const read = (src: string): Promise<string> => {
    const known = templates.get(src);
    if (known !== undefined) return known;
    const template = readFile(join(root, "src/pages", src), "utf8").then((text) => translate(text, t));
    templates.set(src, template);
    return template;
  };
  return Promise.all(list.map(async (page) => {
    const source = await read(page.src);
    const trailData = page.trail ? [breadcrumb(page.trail, locale)] : [];
    const jsonld = page.url === "" ? homeData(locale) : [...trailData, ...(page.jsonld ?? [])];
    const body = fill(page.render ? page.render(source, devices) : source, {
      crumbs: page.trail ? crumbs(page.trail, t) : "",
    });
    return Object.assign({}, page, { jsonld, body });
  }));
}

// Outside English, a model page is noindex: its text comes from the device file.
function deviceEntry(device: Device, reference: Reference, devices: Device[], locale: Locale): Draft {
  const page = devicePage(device, reference, devices, strings(locale));
  return {
    url: page.url,
    nav: "devices",
    title: page.title,
    description: page.description,
    klass: "is-device",
    ...(locale.id === ENGLISH.id ? {} : { noindex: true }),
    jsonld: [breadcrumb(page.breadcrumb, locale), deviceData(device, page, locale)],
    body: page.body,
  };
}

function docPages(docs: Doc[], nav: NavEntry[], docsHome: string, locale: Locale): Draft[] {
  const t = strings(locale);
  return docs.map((doc) => ({
    url: `docs/${doc.slug}/`,
    nav: "docs",
    title: doc.title,
    description: doc.description,
    klass: "is-doc",
    jsonld: [breadcrumb([[t("nav.docs"), docsHome], [doc.title, `docs/${doc.slug}/`]], locale)],
    body: docPage(doc, nav, t),
  }));
}

function refPage(reference: Reference, api: Api, nav: NavEntry[], docsHome: string, locale: Locale): Draft {
  const t = strings(locale);
  return {
    url: REFERENCE_URL,
    nav: "docs",
    title: t("ref.title"),
    description: reference.intro,
    klass: "is-doc",
    jsonld: [breadcrumb([[t("nav.docs"), docsHome], [t("ref.title"), REFERENCE_URL]], locale)],
    body: referencePage(reference, api, nav, t),
  };
}

// A page that moved keeps its old address, which a published README links to.
const MOVED: Record<string, string> = { "reference/": REFERENCE_URL, "devices/lan/": "devices/" };

// One page serves every missing address, so it stays in English.
function notFound(): Page {
  const t = strings(ENGLISH);
  return {
    locale: ENGLISH,
    url: "404.html",
    nav: "",
    title: t("404.title"),
    noindex: true,
    body: `<section class="slab"><h1>${t("404.title")}</h1><p class="lede">${t("404.text")}</p></section>`,
  };
}

// Two renames: `dist/` is missing for microseconds, not for a whole build.
// `previous` carries the pid: two builds that publish at once (`npm run dev`
// beside `tools/qa-site.sh`) must not delete the directory of the other.
async function publish(): Promise<void> {
  const previous = `${dist}.previous-${process.pid}`;
  await rm(previous, { recursive: true, force: true });
  if (existsSync(dist)) await rename(dist, previous);
  await rename(out, dist);
  await rm(previous, { recursive: true, force: true });
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
