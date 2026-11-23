// `npm run dev`: proxy to govee-server, rebuild on change, reload the open pages.

import { type ChildProcess, spawn } from "node:child_process";
import { watch } from "node:fs";
import { type IncomingMessage, type ServerResponse, createServer, request } from "node:http";
import { createServer as createNetServer } from "node:net";
import { join } from "node:path";
import * as esbuild from "esbuild";
import { clean, copyStatic, options, src } from "./build.ts";

const PORT = Number(process.env["PORT"] ?? 4080);
const API_PORT = Number(process.env["API_PORT"] ?? 4081);
const LIVE_PATH = "/__reload";

// Added when a page is sent, so that `dist/` stays the build that ships.
const CLIENT = `<script type="module">
const events = new EventSource("${LIVE_PATH}");
let lost = false;
events.addEventListener("error", () => { lost = true; });
events.addEventListener("open", () => { if (lost) location.reload(); });
events.addEventListener("message", ({ data }) => {
  if (data === "locale") { dispatchEvent(new Event("studio:locale")); return; }
  const links = document.querySelectorAll('link[rel="stylesheet"]');
  if (data !== "css" || links.length === 0) { location.reload(); return; }
  for (const link of links) link.href = link.getAttribute("href").split("?")[0] + "?" + Date.now();
});
</script>`;

const WAITING = `<!doctype html><meta charset="utf-8"><meta http-equiv="refresh" content="1">
<title>Studio</title><body style="background:#0a0d14;color:#9694a0;font:14px monospace;padding:2em">
govee-server is starting…</body>`;

const clients = new Set<ServerResponse>();

type Change = "css" | "locale" | "page";

function notify(change: Change): void {
  for (const client of clients) client.write(`data: ${change}\n\n`);
}

// `retry` shortens the wait of the browser after a restart (3 s by default).
function listen(response: ServerResponse): void {
  response.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-store" });
  response.write("retry: 250\n\n");
  clients.add(response);
  response.on("close", () => {
    clients.delete(response);
  });
}

function relay(upstream: IncomingMessage, response: ServerResponse): void {
  const headers = { ...upstream.headers, "cache-control": "no-store" };
  if (upstream.headers["content-type"]?.startsWith("text/html") !== true) {
    response.writeHead(upstream.statusCode ?? 502, headers);
    upstream.pipe(response);
    return;
  }
  const chunks: Buffer[] = [];
  upstream.on("data", (chunk: Buffer) => {
    chunks.push(chunk);
  });
  upstream.on("end", () => {
    const html = Buffer.concat(chunks).toString("utf8").replace("</body>", `${CLIENT}\n</body>`);
    delete headers["content-length"];
    response.writeHead(upstream.statusCode ?? 502, headers);
    response.end(html);
  });
}

function proxy(incoming: IncomingMessage, response: ServerResponse): void {
  const outgoing = request(
    { host: "127.0.0.1", port: API_PORT, path: incoming.url, method: incoming.method, headers: incoming.headers },
    (upstream) => {
      relay(upstream, response);
    },
  );
  outgoing.on("error", () => {
    response.writeHead(503, { "Content-Type": "text/html; charset=utf-8" });
    response.end(WAITING);
  });
  incoming.pipe(outgoing);
}

function server(): ChildProcess {
  const manifest = join(import.meta.dirname, "../../packages/rust/Cargo.toml");
  const args = ["run", "--manifest-path", manifest, "-p", "govee-toolkit-server", "--"];
  return spawn("cargo", [...args, "--listen", `127.0.0.1:${API_PORT}`, "--web", "dist"], {
    cwd: import.meta.dirname,
    stdio: "inherit",
  });
}

function free(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const probe = createNetServer();
    probe.once("error", () => {
      resolve(false);
    });
    probe.listen(port, "127.0.0.1", () => {
      probe.close(() => {
        resolve(true);
      });
    });
  });
}

const ports = [PORT, API_PORT];
const results = await Promise.all(ports.map((port) => free(port)));
const taken = ports.filter((_port, i) => !(results[i] ?? false));
if (taken.length > 0) {
  console.error(`studio dev: port ${taken.join(", ")} in use. Stop the other dev server or govee-server first.`);
  process.exit(1);
}

await clean();
const context = await esbuild.context(options(true));
await context.rebuild();
await copyStatic();

function kind(files: string[]): Change {
  if (files.every((f) => f.endsWith(".css"))) return "css";
  if (files.every((f) => f.startsWith("assets/locales"))) return "locale";
  return "page";
}

// One change often fires several events: wait for the burst to end.
let timer: NodeJS.Timeout | undefined;
let pending = new Set<string>();
watch(src, { recursive: true }, (_event, file) => {
  if (file !== null) pending.add(file);
  clearTimeout(timer);
  timer = setTimeout(() => {
    const files = [...pending];
    pending = new Set();
    void (async () => {
      if (files.some((f) => f.endsWith(".ts") || f.endsWith(".css"))) await context.rebuild().catch(() => null);
      await copyStatic();
      notify(kind(files));
    })();
  }, 50);
});

const api = server();
createServer((incoming, response) => {
  if (incoming.url === LIVE_PATH) listen(response);
  else proxy(incoming, response);
}).listen(PORT, "127.0.0.1", () => {
  console.log(`studio dev: http://127.0.0.1:${PORT}`);
});

// A closed terminal sends SIGHUP: without a handler, govee-server stays on API_PORT.
let stopping = false;
function stop(code: number): void {
  stopping = true;
  api.kill("SIGINT");
  void context.dispose();
  process.exit(code);
}
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
  process.on(signal, () => {
    stop(0);
  });
}
api.on("exit", (code) => {
  if (stopping) return;
  console.error(`studio dev: govee-server stopped (code ${code ?? "none"})`);
  stop(1);
});
