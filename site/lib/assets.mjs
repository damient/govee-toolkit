// The stylesheet and the script. Both are bundled from an entry that imports
// the rest, so a file stays small and the page still loads one of each.

import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { build } from "esbuild";
import { root } from "./config.mjs";

/**
 * Writes the script into `out` and returns the stylesheet, which the layout
 * inlines: an external one costs a round trip before the first paint.
 */
export async function assets(out) {
  const [css, js] = await Promise.all([
    bundle("src/assets/css/site.css"),
    bundle("src/assets/js/site.js"),
  ]);
  await mkdir(join(out, "assets/js"), { recursive: true });
  await writeFile(join(out, "assets/js/site.js"), js);
  return css;
}

// The font addresses are absolute, so they stay as they stand and no font is
// copied into the bundle.
async function bundle(entry) {
  const result = await build({
    entryPoints: [join(root, entry)],
    bundle: true,
    minify: true,
    format: "esm",
    target: "es2022",
    external: ["/assets/*"],
    write: false,
  });
  return result.outputFiles[0].text;
}
