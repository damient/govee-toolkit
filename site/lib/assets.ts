// Both are bundled from an entry that imports the rest, so a source file stays
// small and the page still loads one stylesheet and one script.

import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { build } from "esbuild";
import { root } from "./config.ts";

/**
 * Writes the script into `out` and returns the stylesheet, which the layout
 * inlines: an external one costs a round trip before the first paint.
 */
export async function assets(out: string): Promise<string> {
  const [css, js] = await Promise.all([
    bundle("src/assets/css/site.css"),
    bundle("src/assets/ts/site.ts"),
  ]);
  await mkdir(join(out, "assets/js"), { recursive: true });
  await writeFile(join(out, "assets/js/site.js"), js);
  return css;
}

// The font addresses are absolute, so they stay as they stand and no font is
// copied into the bundle.
async function bundle(entry: string): Promise<string> {
  const result = await build({
    entryPoints: [join(root, entry)],
    bundle: true,
    minify: true,
    format: "esm",
    target: "es2022",
    external: ["/assets/*"],
    write: false,
  });
  const file = result.outputFiles[0];
  if (!file) throw new Error(`${entry}: esbuild wrote no output`);
  return file.text;
}
