import { createReadStream, existsSync, statSync } from "node:fs";
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

/** Serves `dist/` on `PORT`, 8787 by default. */
export function serve(): void {
  const port = Number(process.env.PORT ?? 8787);
  createServer((request, response) => {
    const path = decodeURIComponent(new URL(request.url ?? "/", "http://localhost").pathname);
    let file = join(dist, path);
    if (!file.startsWith(dist)) return send(response, 403, "Forbidden");
    if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
    let ext = extname(file);
    if (!existsSync(file)) {
      file = join(dist, "404.html");
      if (!existsSync(file)) return send(response, 404, "Not found");
      // A missing stylesheet must fail as a stylesheet, not arrive as HTML
      // the browser parses.
      ext = ".html";
      response.statusCode = 404;
    }
    response.setHeader("Content-Type", TYPES[ext] ?? "application/octet-stream");
    response.setHeader("Cache-Control", "no-store");
    createReadStream(file).pipe(response);
  })
    // Without this, a port already taken raises an unhandled error event, and
    // a reader takes the stack trace for a watcher that does not work.
    .on("error", (error: NodeJS.ErrnoException) => {
      if (error.code !== "EADDRINUSE") throw error;
      console.error(`port ${port} is taken. Another server already serves the site.`);
      console.error(`Stop it with: kill $(lsof -ti :${port} -sTCP:LISTEN)`);
      console.error(`Or serve on another port: PORT=8788 npm run dev`);
      process.exit(1);
    })
    .listen(port, () => console.log(`http://localhost:${port}`));
}

function send(response: ServerResponse, code: number, body: string): void {
  response.statusCode = code;
  response.end(body);
}
