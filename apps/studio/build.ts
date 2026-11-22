// The fonts and the icons come from the site: one copy of each file.

import { cp, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import * as esbuild from "esbuild";

const root = import.meta.dirname;
export const src = join(root, "src");
export const dist = join(root, "dist");
const site = join(root, "../../site/src/assets");

const FONTS = ["ibm-plex-sans-latin.woff2", "ibm-plex-mono-500-latin.woff2", "ibm-plex-mono-600-latin.woff2", "OFL.txt"];

/** Copy the files that esbuild does not bundle into `dist/`. */
export async function copyStatic(): Promise<void> {
  await mkdir(join(dist, "fonts"), { recursive: true });
  await Promise.all([
    cp(join(src, "index.html"), join(dist, "index.html")),
    cp(join(src, "style.css"), join(dist, "style.css")),
    cp(join(src, "locales"), join(dist, "locales"), { recursive: true }),
    cp(join(src, "manifest.webmanifest"), join(dist, "manifest.webmanifest")),
    cp(join(site, "favicon.svg"), join(dist, "favicon.svg")),
    cp(join(site, "apple-touch-icon.png"), join(dist, "apple-touch-icon.png")),
    ...FONTS.map((font) => cp(join(site, "fonts", font), join(dist, "fonts", font))),
  ]);
}

/** Empty `dist/`. */
export async function clean(): Promise<void> {
  await rm(dist, { recursive: true, force: true });
  await mkdir(dist, { recursive: true });
}

/** `dev` adds a source map and keeps the bundle readable. */
export function options(dev: boolean): esbuild.BuildOptions {
  return {
    entryPoints: [join(src, "main.ts")],
    outfile: join(dist, "app.js"),
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
