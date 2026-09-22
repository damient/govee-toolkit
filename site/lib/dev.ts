// What `npm run dev` adds to a build: a rebuild on every change, and the
// signal that makes an open page follow it. `node --watch` restarts the whole
// process on a change to `build.ts` or `lib/`, because an imported module is
// cached and a rebuild inside one process reads the old code.

import { watch } from "node:fs";
import { readdir, rm } from "node:fs/promises";
import { basename, dirname, join, sep } from "node:path";
import { catalogPath, lanListPath, root } from "./config.ts";
import type { Change } from "./serve.ts";

const SOURCES = ["src", "content"];
const CSS_DIR = `src${sep}assets${sep}css${sep}`;

/**
 * Calls `rebuild` after a burst of changes under `src/` and `content/`, or to
 * the catalog or the LAN list. The change is `css` where every file that changed is a
 * stylesheet.
 */
export function watchSources(rebuild: (change: Change) => Promise<void>): void {
  let queued: NodeJS.Timeout | undefined;
  let css = true;
  const touch = (path: string) => {
    css &&= path.startsWith(CSS_DIR);
    clearTimeout(queued);
    queued = setTimeout(() => {
      const change = css ? "css" : "page";
      css = true;
      rebuild(change).catch(console.error);
    }, 80);
  };
  for (const dir of SOURCES) {
    watch(join(root, dir), { recursive: true }, (_, file) => touch(join(dir, file ?? "")));
  }
  // The directory and not the file: `xtask` replaces the file, and a watch on
  // the file ends with the file it watched.
  const catalog = basename(catalogPath);
  watch(dirname(catalogPath), (_, file) => {
    if (file === catalog) touch(catalog);
  });
  const lan = basename(lanListPath);
  watch(dirname(lanListPath), (_, file) => {
    if (file === lan) touch(lan);
  });
  console.log("watching src/, content/, the catalog and the LAN list …");
}

/**
 * Deletes the staging directory of every build whose process is gone. A build
 * stopped halfway leaves its directory behind.
 */
export async function sweepStaging(): Promise<void> {
  const stale = (await readdir(root)).filter((name) => {
    const pid = /^\.dist-build-(\d+)$/.exec(name)?.[1];
    return pid !== undefined && !alive(Number(pid));
  });
  await Promise.all(stale.map((name) => rm(join(root, name), { recursive: true, force: true })));
}

function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    // `EPERM`: the process exists and belongs to another user.
    return (error as NodeJS.ErrnoException).code === "EPERM";
  }
}
