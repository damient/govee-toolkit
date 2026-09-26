// The server, served over stdio. stdout carries the JSON-RPC stream: every log
// goes to stderr.

import { readFileSync } from "node:fs";

import { McpServer } from "@modelcontextprotocol/server";
import { serveStdio } from "@modelcontextprotocol/server/stdio";
import { z } from "zod";

import { registerResources } from "./resources.ts";
import { closeSdk } from "./sdk.ts";
import { registerCatalogTools } from "./tools/catalog.ts";
import { registerControlTools } from "./tools/control.ts";

const { name, version } = z
  .object({ name: z.string(), version: z.string() })
  .parse(JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")));

/** One server with every tool and resource registered. The tests connect a client to it. */
export function createServer(): McpServer {
  const server = new McpServer({ name, version }, { capabilities: { tools: {}, resources: {} } });
  registerCatalogTools(server);
  registerControlTools(server);
  registerResources(server);
  return server;
}

/** Serves over the process stdio until stdin closes or a signal arrives. */
export function serve(): void {
  const handle = serveStdio(createServer, {
    onerror: (error) => {
      console.error(error);
    },
  });
  const shutdown = async (): Promise<never> => {
    // One chain, so each hook starts after the one before, and a failure stops no other hook.
    await [() => handle.close(), closeSdk].reduce(
      (previous, hook) =>
        previous.then(hook).catch((error: unknown) => {
          console.error("shutdown:", error);
        }),
      Promise.resolve(),
    );
    process.exit(0);
  };
  process.stdin.on("close", () => void shutdown());
  process.on("SIGINT", () => void shutdown());
  process.on("SIGTERM", () => void shutdown());
}
