import { readFileSync, readdirSync } from "node:fs";
import { cp, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import * as esbuild from "esbuild";

const root = import.meta.dirname;
export const src = join(root, "src");
export const dist = join(root, "dist");
const site = join(root, "../../site/src/assets");
const familyIcons = join(root, "../../site/src/icons/family");

const FONTS = ["ibm-plex-sans-latin.woff2", "ibm-plex-mono-500-latin.woff2", "ibm-plex-mono-600-latin.woff2", "OFL.txt"];

export async function copyStatic(): Promise<void> {
  await mkdir(join(dist, "fonts"), { recursive: true });
  await Promise.all([
    cp(join(src, "index.html"), join(dist, "index.html")),
    cp(join(src, "assets/locales"), join(dist, "locales"), { recursive: true }),
    cp(join(src, "manifest.webmanifest"), join(dist, "manifest.webmanifest")),
    cp(join(site, "favicon.svg"), join(dist, "favicon.svg")),
    cp(join(site, "apple-touch-icon.png"), join(dist, "apple-touch-icon.png")),
    ...FONTS.map((font) => cp(join(site, "fonts", font), join(dist, "fonts", font))),
  ]);
}

export async function clean(): Promise<void> {
  await rm(dist, { recursive: true, force: true });
  await mkdir(dist, { recursive: true });
}

/** The version in `package.json`, which `src/core/version.ts` reads. */
function version(): string {
  const manifest: unknown = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  if (typeof manifest === "object" && manifest !== null && "version" in manifest && typeof manifest.version === "string") {
    return manifest.version;
  }
  throw new Error("package.json has no version");
}

/** The markup inside `<svg>` of each family icon of the site, by family. */
function families(): Record<string, string> {
  const icons: Record<string, string> = {};
  for (const file of readdirSync(familyIcons)) {
    if (!file.endsWith(".svg")) continue;
    const source = readFileSync(join(familyIcons, file), "utf8");
    const open = /<svg\b[^>]*>/u.exec(source);
    if (open === null) throw new Error(`${file}: no <svg> element`);
    const inner = source.slice(open.index + open[0].length, source.lastIndexOf("</svg>"));
    icons[file.slice(0, -4)] = inner.trim().replaceAll(/\s*\n\s*/gu, "");
  }
  return icons;
}

export function options(dev: boolean): esbuild.BuildOptions {
  return {
    define: { STUDIO_VERSION: JSON.stringify(version()), STUDIO_FAMILY_ICONS: JSON.stringify(families()) },
    entryPoints: [join(src, "main.ts")],
    outfile: join(dist, "app.js"),
    external: ["/fonts/*"],
    bundle: true,
    format: "esm",
    target: "es2022",
    minify: !dev,
    sourcemap: dev,
    logLevel: "info",
  };
}

if (import.meta.main) {
  await clean();
  await esbuild.build(options(false));
  await copyStatic();
}
