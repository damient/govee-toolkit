import { createReadStream, existsSync, statSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { type ServerResponse, createServer } from "node:http";
import { extname, join } from "node:path";
import { dist } from "./config.ts";

const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
  ".json": "application/json",
  ".xml": "application/xml",
  ".txt": "text/plain; charset=utf-8",
};

const LIVE_PATH = "/__reload";

/** What changed: `css` swaps the inlined stylesheet, `page` reloads. */
export type Change = "css" | "page";

// Added when the page is sent, so that `dist/` stays the site that ships. An
// `open` after an `error` is a server that `node --watch` restarted.
const CLIENT = `<script type="module">
const events = new EventSource("${LIVE_PATH}");
let lost = false;
events.addEventListener("error", () => { lost = true; });
events.addEventListener("open", () => { if (lost) location.reload(); });
events.addEventListener("message", async ({ data }) => {
  const style = document.querySelector("style");
  if (data !== "css" || !style) return location.reload();
  const page = await fetch(location.href, { cache: "no-store" }).then((r) => r.text());
  const next = new DOMParser().parseFromString(page, "text/html").querySelector("style");
  if (next) style.textContent = next.textContent;
});
</script>`;

const clients = new Set<ServerResponse>();

/** Tells every open page that the site changed. */
export function notify(change: Change): void {
  for (const client of clients) client.write(`data: ${change}\n\n`);
}

/**
 * Serves `dist/` on `PORT`, 8787 by default. `live` adds the reload channel
 * and the script that listens to it.
 */
export function serve({ live = false } = {}): void {
  const port = Number(process.env.PORT ?? 8787);
  createServer((request, response) => {
    const path = decodeURIComponent(new URL(request.url ?? "/", "http://localhost").pathname);
    if (live && path === LIVE_PATH) { listen(response); return; }
    let file = join(dist, path);
    if (!file.startsWith(dist)) { send(response, 403, "Forbidden"); return; }
    if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
    let ext = extname(file);
    if (!existsSync(file)) {
      file = join(dist, "404.html");
      if (!existsSync(file)) { send(response, 404, "Not found"); return; }
      // A missing stylesheet must not arrive as HTML that the browser parses.
      ext = ".html";
      response.statusCode = 404;
    }
    response.setHeader("Content-Type", TYPES[ext] ?? "application/octet-stream");
    response.setHeader("Cache-Control", "no-store");
    if (live && ext === ".html") {
      readFile(file, "utf8")
        .then((html) => response.end(html.replace("</body>", `${CLIENT}\n</body>`)))
        .catch(() => { send(response, 500, "Read failed"); });
      return;
    }
    createReadStream(file).pipe(response);
  })
    // A port already taken otherwise prints a stack trace.
    .on("error", (error: NodeJS.ErrnoException) => {
      if (error.code !== "EADDRINUSE") throw error;
      console.error(`port ${port} is taken. Another server already serves the site.`);
      console.error(`Stop it with: kill $(lsof -ti :${port} -sTCP:LISTEN)`);
      console.error(`Or serve on another port: PORT=8788 npm run dev`);
      process.exit(1);
    })
    .listen(port, () => { console.log(`http://localhost:${port}`); });
}

// `retry` shortens the wait of the browser after a restart (3 s by default).
function listen(response: ServerResponse): void {
  response.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-store",
    Connection: "keep-alive",
  });
  response.write("retry: 250\n\n");
  clients.add(response);
  response.on("close", () => {
    clients.delete(response);
  });
}

function send(response: ServerResponse, code: number, body: string): void {
  response.statusCode = code;
  response.end(body);
}
