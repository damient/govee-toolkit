// The home page and the product pages Studio, DMX and SDK, in every language.
// English also serves the previous home page (`index.html`) at `old/`.

import { codeBlock, highlight } from "./code.ts";
import { deskHtml } from "./desk.ts";
import { fill } from "./html.ts";
import { cardIcons } from "./icons.ts";
import type { T } from "./locales.ts";
import type { Crumb } from "./types.ts";

/** One hand-written page of `src/pages/`, as `build.ts` reads it. */
export interface ProductSource {
  src: string;
  url: string;
  nav: string;
  title: string | null;
  klass?: string;
  noindex?: boolean;
  render?: (template: string) => string;
  trail?: Crumb[];
  description?: string;
}

/** The pages of one language. */
export function productSources(t: T): ProductSource[] {
  const parts = (template: string) =>
    fill(template, { ...cardIcons(), sdk_tabs: sdkTabs(t("docs.copy"), t("ref.language")), dmx_desk: deskHtml(t) });
  return [
    { src: "home.html", url: "", nav: "home", title: null, klass: "is-home is-product has-beam", render: parts },
    ...(t.lang === "en" ? [{ src: "index.html", url: "old/", nav: "home", title: null, klass: "is-home has-beam", noindex: true }] : []),
    {
      src: "studio.html",
      url: "studio/",
      nav: "studio",
      title: "Studio",
      klass: "is-product has-beam",
      render: parts,
      trail: [["Studio", "studio/"]],
      description: t("pstudio.description"),
    },
    {
      src: "dmx.html",
      url: "dmx/",
      nav: "dmx",
      title: t("home3.dmx_name"),
      klass: "is-product has-beam",
      render: parts,
      trail: [[t("home3.dmx_name"), "dmx/"]],
      description: t("pdmx.description"),
    },
    {
      src: "sdk.html",
      url: "sdk/",
      nav: "developers",
      title: t("nav.developers"),
      klass: "is-product has-beam",
      render: parts,
      trail: [[t("nav.developers"), "sdk/"]],
      description: t("psdk.description"),
    },
  ];
}

const SNIPPETS: [label: string, language: string, install: string, code: string][] = [
  ["Python", "python", "pip install govee-toolkit", `from govee_toolkit import Govee

govee = await Govee.start()
devices = await govee.devices()
await devices.power(True)`],
  ["Node.js", "javascript", "npm install govee-toolkit", `import { Govee } from "govee-toolkit"

const govee = await Govee.start()
const devices = await govee.devices()
await devices.power(true)`],
  ["Rust", "rust", "cargo add govee-toolkit", `use govee_toolkit::{Config, Filter, Govee};

let govee = Govee::start(Config::load()?).await?;
let devices = govee.devices(Filter::all(), None).await?;
devices.power(true).await;`],
  ["CLI", "bash", "cargo install govee-toolkit-cli", `govee scan
govee on DEVICE
govee color DEVICE "#ff3d00"`],
];

const slug = (label: string) => label.toLowerCase().replaceAll(/[^a-z]/gu, "");

// The markup of `lib/tabs.ts`, so `choices.ts` wires it.
function sdkTabs(copy: string, listLabel: string): string {
  const buttons = SNIPPETS.map(([label], i) =>
    `<button type="button" role="tab" id="tab-sdk-${slug(label)}" aria-controls="pane-sdk-${slug(label)}"`
    + ` data-tab="${slug(label)}" aria-selected="${i === 0}" tabindex="${i === 0 ? "0" : "-1"}">${label}</button>`).join("");
  const panes = SNIPPETS.map(([label, language, install, code], i) =>
    `<div class="tab-pane" role="tabpanel" id="pane-sdk-${slug(label)}" aria-labelledby="tab-sdk-${slug(label)}"`
    + ` data-tab="${slug(label)}"${i === 0 ? "" : " hidden"}>\n`
    + codeBlock(`<span class="prompt">$</span> ${install}`, install, copy)
    + codeBlock(highlight(code, language), code, copy)
    + "</div>").join("\n");
  return `<div class="choice sdk-code" data-tabs="sdk">\n<div class="tabs" role="tablist" aria-label="${listLabel}">${buttons}</div>\n${panes}\n</div>`;
}
