// Builds src/ into dist/. `--watch` rebuilds on every change.
//
// The fonts and the icon come from the site, so the two surfaces carry one
// copy of each file.

import { cp, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import * as esbuild from "esbuild";

const root = import.meta.dirname;
const src = join(root, "src");
const dist = join(root, "dist");
const site = join(root, "../../site/src/assets");
const watch = process.argv.includes("--watch");

await rm(dist, { recursive: true, force: true });
await mkdir(dist, { recursive: true });

async function copyStatic(): Promise<void> {
  await cp(join(src, "index.html"), join(dist, "index.html"));
  await cp(join(src, "style.css"), join(dist, "style.css"));
  await cp(join(site, "favicon.svg"), join(dist, "favicon.svg"));
  await mkdir(join(dist, "fonts"), { recursive: true });
  for (const font of FONTS) {
    await cp(join(site, "fonts", font), join(dist, "fonts", font));
  }
}

// The faces that style.css declares, and their license.
const FONTS = ["ibm-plex-sans-latin.woff2", "ibm-plex-mono-500-latin.woff2", "OFL.txt"];

const copyPlugin: esbuild.Plugin = {
  name: "copy-static",
  setup(build) {
    build.onEnd(async (result) => {
      if (result.errors.length === 0) await copyStatic();
    });
  },
};

const options: esbuild.BuildOptions = {
  entryPoints: [join(src, "main.ts")],
  outfile: join(dist, "app.js"),
  bundle: true,
  format: "esm",
  target: "es2022",
  minify: !watch,
  sourcemap: watch,
  logLevel: "info",
  plugins: [copyPlugin],
};

if (watch) {
  const context = await esbuild.context(options);
  await context.watch();
} else {
  await esbuild.build(options);
}
